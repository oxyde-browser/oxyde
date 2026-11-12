//! Bac à sable sous Windows : jeton restreint, niveau d'intégrité bas, objet
//! de travail, politiques d'atténuation.
//!
//! Tout le code `unsafe` du crate est ici. Chaque appel au système est
//! enveloppé au plus près, et les poignées vivent dans des [`OwnedHandle`] :
//! aucune n'est fermée deux fois ni oubliée, quel que soit le chemin de
//! sortie.

#![allow(unsafe_code, reason = "appels à l'API Win32, sans équivalent sûr")]

use std::ffi::c_void;
use std::io::{self, PipeReader, PipeWriter};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsHandle, AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    ERROR_INSUFFICIENT_BUFFER, GetLastError, HANDLE, HANDLE_FLAG_INHERIT, SetHandleInformation,
};
use windows_sys::Win32::Security::{
    CreateRestrictedToken, CreateWellKnownSid, DISABLE_MAX_PRIVILEGE, GetLengthSid,
    GetTokenInformation, SECURITY_MAX_SID_SIZE, SID_AND_ATTRIBUTES, SetTokenInformation,
    TOKEN_ADJUST_DEFAULT, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_GROUPS,
    TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenGroups, TokenIntegrityLevel, WELL_KNOWN_SID_TYPE,
    WinBuiltinUsersSid, WinLowLabelSid, WinRestrictedCodeSid, WinWorldSid,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOB_OBJECT_UILIMIT_DESKTOP, JOB_OBJECT_UILIMIT_DISPLAYSETTINGS, JOB_OBJECT_UILIMIT_EXITWINDOWS,
    JOB_OBJECT_UILIMIT_GLOBALATOMS, JOB_OBJECT_UILIMIT_HANDLES, JOB_OBJECT_UILIMIT_READCLIPBOARD,
    JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS, JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
    JOBOBJECT_BASIC_UI_RESTRICTIONS, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicUIRestrictions, JobObjectExtendedLimitInformation, SetInformationJobObject,
};
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, CreateProcessAsUserW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess, INFINITE, InitializeProcThreadAttributeList,
    OpenProcessToken, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROCESS_INFORMATION,
    ProcessChildProcessPolicy, ProcessDynamicCodePolicy, ProcessExtensionPointDisablePolicy,
    ProcessImageLoadPolicy, ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW,
    SetProcessMitigationPolicy, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
};

/// Attribut d'un groupe de jeton : identifiant de la session d'ouverture.
const SE_GROUP_LOGON_ID: u32 = 0xC000_0000;
/// Attribut d'un groupe de jeton : étiquette d'intégrité.
const SE_GROUP_INTEGRITY: u32 = 0x20;

/// Erreur du dernier appel système, précédée de ce qu'on tentait.
fn last_error(what: &str) -> io::Error {
    let error = io::Error::last_os_error();
    io::Error::new(error.kind(), format!("{what} : {error}"))
}

/// Prend possession d'une poignée que le système vient de rendre.
///
/// # Safety
///
/// `handle` doit être une poignée ouverte, valide, que personne d'autre ne
/// fermera.
unsafe fn own(handle: HANDLE) -> OwnedHandle {
    // SAFETY: garanti par l'appelant.
    unsafe { OwnedHandle::from_raw_handle(handle) }
}

/// Identifiant de sécurité, dans un tampon aligné comme l'exige le système.
#[repr(C, align(4))]
#[derive(Clone, Copy)]
struct Sid([u8; SECURITY_MAX_SID_SIZE as usize]);

impl Sid {
    fn as_ptr(&self) -> *mut c_void {
        self.0.as_ptr().cast_mut().cast()
    }

    fn well_known(kind: WELL_KNOWN_SID_TYPE) -> io::Result<Self> {
        let mut sid = Self([0; SECURITY_MAX_SID_SIZE as usize]);
        let mut size = SECURITY_MAX_SID_SIZE;
        // SAFETY: `sid` est un tampon de `size` octets, accessible en
        // écriture et aligné ; le système n'y écrit pas au-delà de `size`.
        let ok = unsafe {
            CreateWellKnownSid(kind, null_mut(), sid.0.as_mut_ptr().cast(), &raw mut size)
        };
        if ok == 0 {
            return Err(last_error("identifiant de sécurité indisponible"));
        }
        Ok(sid)
    }
}

/// Jeton du processus courant, avec les droits qu'il faut pour en tirer un
/// jeton restreint.
fn process_token() -> io::Result<OwnedHandle> {
    let mut token: HANDLE = null_mut();
    let access = TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY | TOKEN_QUERY | TOKEN_ADJUST_DEFAULT;
    // SAFETY: sans condition préalable ; rend une pseudo-poignée toujours valide.
    let current = unsafe { GetCurrentProcess() };
    // SAFETY: `current` est valide ; `token` reçoit une poignée que nous
    // prenons en charge juste après.
    let ok = unsafe { OpenProcessToken(current, access, &raw mut token) };
    if ok == 0 {
        return Err(last_error("jeton du processus inaccessible"));
    }
    // SAFETY: l'appel a réussi : `token` est une poignée neuve, à nous.
    Ok(unsafe { own(token) })
}

/// Identifiant de la session d'ouverture de `token` : il donne accès au
/// bureau et à la station de fenêtres, pas aux fichiers de l'utilisateur.
fn logon_sid(token: &OwnedHandle) -> io::Result<Option<Sid>> {
    let mut needed = 0u32;
    // SAFETY: tampon nul de longueur nulle : l'appel ne fait que rendre la
    // taille nécessaire dans `needed`.
    unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenGroups,
            null_mut(),
            0,
            &raw mut needed,
        );
    }
    // SAFETY: lecture du code d'erreur du fil courant.
    if needed == 0 || unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
        return Err(last_error("groupes du jeton illisibles"));
    }
    let bytes = needed as usize;
    // Tampon de `u64` : aligné pour `TOKEN_GROUPS`, qui contient des pointeurs.
    let mut buffer = vec![0u64; bytes.div_ceil(size_of::<u64>())];
    // SAFETY: `buffer` fait au moins `needed` octets, accessibles en écriture.
    let ok = unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenGroups,
            buffer.as_mut_ptr().cast(),
            needed,
            &raw mut needed,
        )
    };
    if ok == 0 {
        return Err(last_error("groupes du jeton illisibles"));
    }
    let groups = buffer.as_ptr().cast::<TOKEN_GROUPS>();
    // SAFETY: l'appel a réussi : le tampon commence par un `TOKEN_GROUPS`
    // valide, et il est aligné sur 8 octets.
    let count = unsafe { (*groups).GroupCount } as usize;
    // Le tableau annoncé doit tenir dans le tampon que nous avons fourni.
    let first = std::mem::offset_of!(TOKEN_GROUPS, Groups);
    let fits = count
        .checked_mul(size_of::<SID_AND_ATTRIBUTES>())
        .and_then(|array| array.checked_add(first))
        .is_some_and(|end| end <= buffer.len() * size_of::<u64>());
    if !fits {
        return Err(io::Error::other("groupes du jeton incohérents"));
    }
    // SAFETY: le tampon commence par un `TOKEN_GROUPS` valide : l'adresse de
    // son champ `Groups` se calcule sans rien lire.
    let first_entry = unsafe { &raw const (*groups).Groups }.cast::<SID_AND_ATTRIBUTES>();
    // SAFETY: `count` éléments tiennent dans le tampon (vérifié ci-dessus),
    // et le système les a initialisés.
    let entries = unsafe { std::slice::from_raw_parts(first_entry, count) };
    for entry in entries {
        if entry.Attributes & SE_GROUP_LOGON_ID != SE_GROUP_LOGON_ID || entry.Sid.is_null() {
            continue;
        }
        // SAFETY: `entry.Sid` désigne un identifiant valide, rendu par le
        // système dans le tampon encore vivant.
        let len = unsafe { GetLengthSid(entry.Sid) } as usize;
        let mut sid = Sid([0; SECURITY_MAX_SID_SIZE as usize]);
        if len == 0 || len > sid.0.len() {
            continue;
        }
        // SAFETY: `len` octets lisibles à la source (longueur donnée par le
        // système) et inscriptibles à la destination (vérifié ci-dessus) ;
        // les deux zones sont distinctes.
        unsafe { std::ptr::copy_nonoverlapping(entry.Sid.cast::<u8>(), sid.0.as_mut_ptr(), len) };
        return Ok(Some(sid));
    }
    Ok(None)
}

/// Jeton pour le processus de rendu, tiré de celui du navigateur :
///
/// - tous les privilèges retirés ;
/// - accès restreint aux objets ouverts à tout le monde, aux utilisateurs
///   ou à la session — donc ni les fichiers du profil de l'utilisateur, ni
///   son registre ;
/// - niveau d'intégrité bas : aucune écriture vers ce que l'utilisateur
///   peut modifier.
fn restricted_token() -> io::Result<OwnedHandle> {
    let token = process_token()?;
    let mut sids = vec![
        Sid::well_known(WinBuiltinUsersSid)?,
        Sid::well_known(WinWorldSid)?,
        Sid::well_known(WinRestrictedCodeSid)?,
    ];
    if let Some(logon) = logon_sid(&token)? {
        sids.push(logon);
    }
    let restricting: Vec<SID_AND_ATTRIBUTES> = sids
        .iter()
        .map(|sid| SID_AND_ATTRIBUTES {
            Sid: sid.as_ptr(),
            Attributes: 0,
        })
        .collect();
    let count = u32::try_from(restricting.len()).map_err(io::Error::other)?;
    let mut restricted: HANDLE = null_mut();
    // SAFETY: `token` est ouvert ; `restricting` désigne `count` entrées
    // dont les identifiants (`sids`) vivent jusqu'à la fin de la fonction ;
    // `restricted` reçoit une poignée prise en charge juste après.
    let ok = unsafe {
        CreateRestrictedToken(
            token.as_raw_handle(),
            DISABLE_MAX_PRIVILEGE,
            0,
            null(),
            0,
            null(),
            count,
            restricting.as_ptr(),
            &raw mut restricted,
        )
    };
    if ok == 0 {
        return Err(last_error("jeton restreint impossible à créer"));
    }
    // SAFETY: l'appel a réussi : `restricted` est une poignée neuve, à nous.
    let restricted = unsafe { own(restricted) };

    let low = Sid::well_known(WinLowLabelSid)?;
    let label = TOKEN_MANDATORY_LABEL {
        Label: SID_AND_ATTRIBUTES {
            Sid: low.as_ptr(),
            Attributes: SE_GROUP_INTEGRITY,
        },
    };
    // SAFETY: `low` est un identifiant valide, vivant pour la durée de l'appel.
    let sid_len = unsafe { GetLengthSid(low.as_ptr()) };
    let size = u32::try_from(size_of::<TOKEN_MANDATORY_LABEL>())
        .map_err(io::Error::other)?
        .saturating_add(sid_len);
    // SAFETY: `label` est une structure valide de la taille annoncée, qui
    // désigne `low`, vivant ; le jeton a été ouvert avec le droit de
    // modifier son étiquette.
    let ok = unsafe {
        SetTokenInformation(
            restricted.as_raw_handle(),
            TokenIntegrityLevel,
            (&raw const label).cast(),
            size,
        )
    };
    if ok == 0 {
        return Err(last_error("niveau d'intégrité impossible à abaisser"));
    }
    Ok(restricted)
}

/// Objet de travail du processus de rendu : il meurt avec le navigateur, ne
/// lance aucun processus, et n'a accès ni au presse-papiers, ni aux réglages
/// du système, ni aux fenêtres des autres.
fn job() -> io::Result<OwnedHandle> {
    // SAFETY: objet anonyme, attributs de sécurité par défaut.
    let job = unsafe { CreateJobObjectW(null(), null()) };
    if job.is_null() {
        return Err(last_error("objet de travail impossible à créer"));
    }
    // SAFETY: l'appel a réussi : `job` est une poignée neuve, à nous.
    let job = unsafe { own(job) };

    // SAFETY: structure faite d'entiers et de pointeurs : la valeur nulle
    // en est une valeur valide.
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
        | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
    limits.BasicLimitInformation.ActiveProcessLimit = 1;
    let size = u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
        .map_err(io::Error::other)?;
    // SAFETY: `limits` est une structure valide de la taille annoncée.
    let ok = unsafe {
        SetInformationJobObject(
            job.as_raw_handle(),
            JobObjectExtendedLimitInformation,
            (&raw const limits).cast(),
            size,
        )
    };
    if ok == 0 {
        return Err(last_error("limites de l'objet de travail refusées"));
    }

    let ui = JOBOBJECT_BASIC_UI_RESTRICTIONS {
        UIRestrictionsClass: JOB_OBJECT_UILIMIT_DESKTOP
            | JOB_OBJECT_UILIMIT_DISPLAYSETTINGS
            | JOB_OBJECT_UILIMIT_EXITWINDOWS
            | JOB_OBJECT_UILIMIT_GLOBALATOMS
            | JOB_OBJECT_UILIMIT_HANDLES
            | JOB_OBJECT_UILIMIT_READCLIPBOARD
            | JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS
            | JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
    };
    let size =
        u32::try_from(size_of::<JOBOBJECT_BASIC_UI_RESTRICTIONS>()).map_err(io::Error::other)?;
    // SAFETY: `ui` est une structure valide de la taille annoncée.
    let ok = unsafe {
        SetInformationJobObject(
            job.as_raw_handle(),
            JobObjectBasicUIRestrictions,
            (&raw const ui).cast(),
            size,
        )
    };
    if ok == 0 {
        return Err(last_error("restrictions d'interface refusées"));
    }
    Ok(job)
}

/// Rend `handle` héritable par un processus enfant.
fn make_inheritable(handle: &impl AsHandle) -> io::Result<()> {
    let raw = handle.as_handle().as_raw_handle();
    // SAFETY: `raw` est une poignée ouverte, empruntée pour la durée de l'appel.
    let ok = unsafe { SetHandleInformation(raw, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) };
    if ok == 0 {
        return Err(last_error("poignée impossible à transmettre"));
    }
    Ok(())
}

/// Liste d'attributs de création de processus : elle borne les poignées dont
/// l'enfant hérite à celles qu'on lui destine.
struct AttributeList {
    /// Mémoire de la liste, alignée sur 8 octets.
    storage: Vec<u64>,
    /// Poignées désignées par la liste : elles doivent vivre aussi longtemps
    /// qu'elle.
    _handles: Box<[HANDLE]>,
}

impl AttributeList {
    fn with_handles(handles: Vec<HANDLE>) -> io::Result<Self> {
        let handles = handles.into_boxed_slice();
        let mut size = 0usize;
        // SAFETY: liste nulle : l'appel ne fait que rendre la taille
        // nécessaire dans `size`.
        unsafe { InitializeProcThreadAttributeList(null_mut(), 1, 0, &raw mut size) };
        if size == 0 {
            return Err(last_error("liste d'attributs impossible à dimensionner"));
        }
        let mut storage = vec![0u64; size.div_ceil(size_of::<u64>())];
        // SAFETY: `storage` fait au moins `size` octets, accessibles en écriture.
        let ok = unsafe {
            InitializeProcThreadAttributeList(storage.as_mut_ptr().cast(), 1, 0, &raw mut size)
        };
        if ok == 0 {
            return Err(last_error("liste d'attributs impossible à créer"));
        }
        let list = Self {
            storage,
            _handles: handles,
        };
        // SAFETY: la liste est initialisée ; la valeur désigne le tableau de
        // poignées, de la taille annoncée, que la liste garde en vie.
        let ok = unsafe {
            UpdateProcThreadAttribute(
                list.storage.as_ptr().cast_mut().cast(),
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                list._handles.as_ptr().cast(),
                std::mem::size_of_val(&*list._handles),
                null_mut(),
                null(),
            )
        };
        if ok == 0 {
            return Err(last_error("poignées de l'enfant refusées"));
        }
        Ok(list)
    }

    fn as_ptr(&self) -> *mut c_void {
        self.storage.as_ptr().cast_mut().cast()
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        // SAFETY: la liste a été initialisée par `with_handles` et n'est
        // détruite qu'ici.
        unsafe { DeleteProcThreadAttributeList(self.as_ptr()) };
    }
}

/// Processus lancé dans le bac à sable.
#[derive(Debug)]
pub(crate) struct Restricted {
    process: OwnedHandle,
    /// Gardé ouvert : sa fermeture tue le processus.
    _job: OwnedHandle,
}

impl Restricted {
    pub(crate) fn kill(&mut self) {
        // SAFETY: `process` est une poignée ouverte sur le processus. Un
        // échec (processus déjà terminé) est sans conséquence.
        unsafe { TerminateProcess(self.process.as_raw_handle(), 1) };
    }

    pub(crate) fn wait(&mut self) {
        // SAFETY: `process` est une poignée ouverte sur le processus.
        unsafe { WaitForSingleObject(self.process.as_raw_handle(), INFINITE) };
    }
}

/// Ligne de commande : chaque argument entre guillemets, suivant les règles
/// de découpage de la bibliothèque C de Microsoft.
fn command_line(executable: &Path, arguments: &[&str]) -> io::Result<Vec<u16>> {
    let mut line: Vec<u16> = Vec::new();
    let mut push = |argument: &[u16]| -> io::Result<()> {
        if argument.contains(&0) {
            return Err(io::Error::other("argument contenant un caractère nul"));
        }
        if !line.is_empty() {
            line.push(u16::from(b' '));
        }
        line.push(u16::from(b'"'));
        let mut backslashes = 0usize;
        for unit in argument {
            if *unit == u16::from(b'\\') {
                backslashes += 1;
            } else {
                if *unit == u16::from(b'"') {
                    // Les barres obliques inverses qui précèdent un
                    // guillemet sont doublées, et le guillemet échappé.
                    line.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes + 1));
                }
                backslashes = 0;
            }
            line.push(*unit);
        }
        // Celles qui précèdent le guillemet fermant sont doublées.
        line.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes));
        line.push(u16::from(b'"'));
        Ok(())
    };
    push(&executable.as_os_str().encode_wide().collect::<Vec<_>>())?;
    for argument in arguments {
        push(&argument.encode_utf16().collect::<Vec<_>>())?;
    }
    line.push(0);
    Ok(line)
}

/// Lance `executable` dans le bac à sable, relié par deux tubes : ce que le
/// parent écrit arrive sur l'entrée standard de l'enfant, et il lit sa
/// sortie standard.
pub(crate) fn spawn(
    executable: &Path,
    arguments: &[&str],
) -> io::Result<(Restricted, PipeWriter, PipeReader)> {
    let token = restricted_token()?;
    let job = job()?;
    let (child_stdin, to_child) = io::pipe()?;
    let (from_child, child_stdout) = io::pipe()?;
    make_inheritable(&child_stdin)?;
    make_inheritable(&child_stdout)?;
    // La sortie d'erreur du parent, pour le journal de l'enfant ; il s'en
    // passe si elle n'existe pas (application sans console).
    let child_stderr = io::stderr()
        .as_handle()
        .try_clone_to_owned()
        .ok()
        .filter(|handle| make_inheritable(handle).is_ok());

    let mut inherited = vec![child_stdin.as_raw_handle(), child_stdout.as_raw_handle()];
    if let Some(stderr) = &child_stderr {
        inherited.push(stderr.as_raw_handle());
    }
    let attributes = AttributeList::with_handles(inherited)?;

    let application: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    if application[..application.len() - 1].contains(&0) {
        return Err(io::Error::other("chemin contenant un caractère nul"));
    }
    let mut line = command_line(executable, arguments)?;

    // SAFETY: structure faite d'entiers et de pointeurs : la valeur nulle
    // en est une valeur valide.
    let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    startup.StartupInfo.cb =
        u32::try_from(size_of::<STARTUPINFOEXW>()).map_err(io::Error::other)?;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = child_stdin.as_raw_handle();
    startup.StartupInfo.hStdOutput = child_stdout.as_raw_handle();
    startup.StartupInfo.hStdError = child_stderr
        .as_ref()
        .map_or(null_mut(), AsRawHandle::as_raw_handle);
    startup.lpAttributeList = attributes.as_ptr();

    // SAFETY: structure de sortie faite d'entiers et de pointeurs.
    let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: `token` est un jeton primaire restreint tiré du nôtre ;
    // `application` et `line` sont des chaînes UTF-16 terminées par un zéro,
    // `line` étant modifiable comme l'exige l'appel ; `startup` est une
    // `STARTUPINFOEXW` complète dont la liste d'attributs et les poignées
    // vivent jusqu'à la fin de la fonction ; `information` reçoit deux
    // poignées prises en charge juste après. L'enfant naît suspendu.
    let ok = unsafe {
        CreateProcessAsUserW(
            token.as_raw_handle(),
            application.as_ptr(),
            line.as_mut_ptr(),
            null(),
            null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED,
            null(),
            null(),
            (&raw const startup).cast(),
            &raw mut information,
        )
    };
    if ok == 0 {
        return Err(last_error("lancement dans le bac à sable impossible"));
    }
    // SAFETY: l'appel a réussi : la poignée du processus est neuve, à nous.
    let process = unsafe { own(information.hProcess) };
    // SAFETY: de même pour celle de son fil principal.
    let thread = unsafe { own(information.hThread) };
    let mut child = Restricted { process, _job: job };

    // L'enfant est encore suspendu : il n'exécute rien avant d'être dans
    // son objet de travail.
    // SAFETY: les deux poignées sont ouvertes.
    let assigned = unsafe {
        AssignProcessToJobObject(child._job.as_raw_handle(), child.process.as_raw_handle())
    };
    if assigned == 0 {
        let error = last_error("objet de travail refusé par le processus");
        child.kill();
        return Err(error);
    }
    // SAFETY: `thread` est le fil principal, suspendu, du processus enfant.
    if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
        let error = last_error("démarrage du processus impossible");
        child.kill();
        return Err(error);
    }
    // Les extrémités de l'enfant se ferment ici : s'il s'arrête, le parent
    // voit la fin du tube.
    Ok((child, to_child, from_child))
}

/// Durcit le processus courant : à appeler par le processus de rendu avant
/// de traiter le moindre contenu.
///
/// - pas de code produit à l'exécution (le moteur JavaScript est un
///   interpréteur) ;
/// - pas de points d'extension du système (bibliothèques injectées par des
///   crochets) ;
/// - pas de bibliothèque chargée depuis un partage réseau ;
/// - pas de processus enfant.
///
/// Renvoie le nom des politiques que le système a refusées.
pub(crate) fn harden() -> Vec<&'static str> {
    let policies = [
        (ProcessDynamicCodePolicy, 1u32, "code dynamique"),
        (ProcessExtensionPointDisablePolicy, 1, "points d'extension"),
        (ProcessImageLoadPolicy, 1, "images distantes"),
        (ProcessChildProcessPolicy, 1, "processus enfants"),
    ];
    let mut refused = Vec::new();
    for (policy, flags, name) in policies {
        // SAFETY: chacune de ces politiques se décrit par une structure
        // d'un seul champ de 32 bits ; `flags` en est une valeur valide, de
        // la taille annoncée.
        let ok = unsafe {
            SetProcessMitigationPolicy(policy, (&raw const flags).cast(), size_of::<u32>())
        };
        if ok == 0 {
            refused.push(name);
        }
    }
    refused
}
