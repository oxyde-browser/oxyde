//! Forme sur le fil des types qu'une page échange avec son embarqueur :
//! quand la page vit dans un processus de rendu, ses requêtes réseau et ses
//! demandes de navigation traversent un canal vers le processus navigateur.

use oxyde_ipc::{Bytes, Decode, DecodeError, Encode, Reader, Writer};
use url::Url;

use crate::{FetchRequest, FetchResponse, FormPost, Navigation};

/// Écrit une adresse.
pub fn encode_url(url: &Url, writer: &mut Writer) {
    url.as_str().encode(writer);
}

/// Lit une adresse.
///
/// # Errors
///
/// [`DecodeError::InvalidValue`] si le texte n'est pas une adresse absolue.
pub fn decode_url(reader: &mut Reader<'_>) -> Result<Url, DecodeError> {
    let text = String::decode(reader)?;
    Url::parse(&text).map_err(|_| DecodeError::InvalidValue)
}

fn encode_bytes(bytes: &[u8], writer: &mut Writer) {
    writer.length(bytes.len());
    writer.raw(bytes);
}

fn decode_bytes(reader: &mut Reader<'_>) -> Result<Vec<u8>, DecodeError> {
    Bytes::decode(reader).map(|bytes| bytes.0)
}

impl Encode for FormPost {
    fn encode(&self, writer: &mut Writer) {
        self.content_type.encode(writer);
        encode_bytes(&self.body, writer);
    }
}

impl Decode for FormPost {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            content_type: String::decode(reader)?,
            body: decode_bytes(reader)?,
        })
    }
}

impl Encode for FetchRequest {
    fn encode(&self, writer: &mut Writer) {
        self.id.encode(writer);
        encode_url(&self.url, writer);
        self.method.encode(writer);
        self.headers.encode(writer);
        encode_bytes(&self.body, writer);
        self.origin.encode(writer);
        encode_url(&self.initiator, writer);
        self.needs_preflight.encode(writer);
    }
}

impl Decode for FetchRequest {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            id: u64::decode(reader)?,
            url: decode_url(reader)?,
            method: String::decode(reader)?,
            headers: Vec::decode(reader)?,
            body: decode_bytes(reader)?,
            origin: Option::decode(reader)?,
            initiator: decode_url(reader)?,
            needs_preflight: bool::decode(reader)?,
        })
    }
}

impl Encode for FetchResponse {
    fn encode(&self, writer: &mut Writer) {
        encode_url(&self.url, writer);
        self.status.encode(writer);
        self.headers.encode(writer);
        encode_bytes(&self.body, writer);
    }
}

impl Decode for FetchResponse {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            url: decode_url(reader)?,
            status: u16::decode(reader)?,
            headers: Vec::decode(reader)?,
            body: decode_bytes(reader)?,
        })
    }
}

impl Encode for Navigation {
    fn encode(&self, writer: &mut Writer) {
        match self {
            Self::Load { url, replace, post } => {
                0u8.encode(writer);
                encode_url(url, writer);
                replace.encode(writer);
                post.encode(writer);
            }
            Self::Reload => 1u8.encode(writer),
            Self::Traverse(delta) => {
                2u8.encode(writer);
                delta.encode(writer);
            }
            Self::SameDocument { url, replace } => {
                3u8.encode(writer);
                encode_url(url, writer);
                replace.encode(writer);
            }
        }
    }
}

impl Decode for Navigation {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::decode(reader)? {
            0 => Ok(Self::Load {
                url: decode_url(reader)?,
                replace: bool::decode(reader)?,
                post: Option::decode(reader)?,
            }),
            1 => Ok(Self::Reload),
            2 => Ok(Self::Traverse(i32::decode(reader)?)),
            3 => Ok(Self::SameDocument {
                url: decode_url(reader)?,
                replace: bool::decode(reader)?,
            }),
            tag => Err(DecodeError::UnknownVariant(tag)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyde_ipc::{from_bytes, to_bytes};

    #[test]
    fn exchange_types_round_trip() {
        let url = Url::parse("https://exemple.test/a?b=1#c").expect("url");
        let request = FetchRequest {
            id: 7,
            url: url.clone(),
            method: "POST".to_owned(),
            headers: vec![("X-Essai".to_owned(), "oui".to_owned())],
            body: vec![0, 1, 2, 255],
            origin: Some("https://exemple.test".to_owned()),
            initiator: url.clone(),
            needs_preflight: true,
        };
        assert_eq!(from_bytes::<FetchRequest>(&to_bytes(&request)), Ok(request));
        let response = FetchResponse {
            url: url.clone(),
            status: 404,
            headers: Vec::new(),
            body: b"absent".to_vec(),
        };
        assert_eq!(
            from_bytes::<FetchResponse>(&to_bytes(&response)),
            Ok(response)
        );
        for navigation in [
            Navigation::Load {
                url: url.clone(),
                replace: true,
                post: Some(FormPost {
                    content_type: "text/plain".to_owned(),
                    body: b"a=1".to_vec(),
                }),
            },
            Navigation::Reload,
            Navigation::Traverse(-2),
            Navigation::SameDocument {
                url: url.clone(),
                replace: false,
            },
        ] {
            assert_eq!(
                from_bytes::<Navigation>(&to_bytes(&navigation)),
                Ok(navigation)
            );
        }
        // Une adresse relative n'est pas acceptée.
        assert_eq!(
            from_bytes::<Navigation>(&[3, 4, 0, 0, 0, b'/', b'a', b'b', b'c', 0]),
            Err(DecodeError::InvalidValue)
        );
    }
}
