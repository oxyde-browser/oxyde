// Extraits des standards HTML (https://html.spec.whatwg.org/), CSSOM
// (https://drafts.csswg.org/cssom/), CSSOM View et « DOM Parsing and
// Serialization », transcrits pour Oxyde. Ce ne sont que des extraits : seuls
// les membres rattachés aux interfaces du DOM de base y figurent, et les
// éléments HTML particuliers (HTMLAnchorElement…) ne sont pas encore décrits.

// HTML : extensions de Document.
partial interface Document {
  [PutForwards=href, LegacyUnforgeable] readonly attribute Location? location;
  attribute USVString domain;
  readonly attribute USVString referrer;
  attribute USVString cookie;
  readonly attribute DOMString lastModified;
  readonly attribute DocumentReadyState readyState;

  [CEReactions] attribute DOMString title;
  [CEReactions] attribute DOMString dir;
  [CEReactions] attribute HTMLElement? body;
  readonly attribute HTMLHeadElement? head;
  [SameObject] readonly attribute HTMLCollection images;
  [SameObject] readonly attribute HTMLCollection embeds;
  [SameObject] readonly attribute HTMLCollection plugins;
  [SameObject] readonly attribute HTMLCollection links;
  [SameObject] readonly attribute HTMLCollection forms;
  [SameObject] readonly attribute HTMLCollection scripts;
  NodeList getElementsByName(DOMString elementName);
  readonly attribute HTMLOrSVGScriptElement? currentScript;

  [CEReactions] Document open(optional DOMString unused1, optional DOMString unused2);
  [CEReactions] undefined close();
  [CEReactions] undefined write(DOMString... text);
  [CEReactions] undefined writeln(DOMString... text);

  readonly attribute WindowProxy? defaultView;
  boolean hasFocus();
  [CEReactions] attribute DOMString designMode;
  [CEReactions] boolean execCommand(DOMString commandId, optional boolean showUI = false, optional DOMString value = "");

  readonly attribute boolean hidden;
  readonly attribute DocumentVisibilityState visibilityState;
};

enum DocumentReadyState { "loading", "interactive", "complete" };
enum DocumentVisibilityState { "visible", "hidden" };

// HTML : interface de base des éléments HTML.
[Exposed=Window]
interface HTMLElement : Element {
  [HTMLConstructor] constructor();

  [CEReactions] attribute DOMString title;
  [CEReactions] attribute DOMString lang;
  [CEReactions] attribute boolean translate;
  [CEReactions] attribute DOMString dir;

  [CEReactions] attribute (boolean or unrestricted double or DOMString)? hidden;
  [CEReactions] attribute boolean inert;
  undefined click();
  [CEReactions] attribute DOMString accessKey;
  readonly attribute DOMString accessKeyLabel;
  [CEReactions] attribute boolean draggable;
  [CEReactions] attribute boolean spellcheck;

  [CEReactions] attribute [LegacyNullToEmptyString] DOMString innerText;
  [CEReactions] attribute [LegacyNullToEmptyString] DOMString outerText;
};

interface mixin HTMLOrSVGElement {
  [SameObject] readonly attribute DOMStringMap dataset;
  attribute DOMString nonce;

  [CEReactions] attribute boolean autofocus;
  [CEReactions] attribute long tabIndex;
  undefined focus(optional FocusOptions options = {});
  undefined blur();
};
HTMLElement includes HTMLOrSVGElement;

// CSSOM : style en ligne.
interface mixin ElementCSSInlineStyle {
  [SameObject, PutForwards=cssText] readonly attribute CSSStyleDeclaration style;
};
HTMLElement includes ElementCSSInlineStyle;

[Exposed=Window]
interface CSSStyleDeclaration {
  [CEReactions] attribute CSSOMString cssText;
  readonly attribute unsigned long length;
  getter CSSOMString item(unsigned long index);
  CSSOMString getPropertyValue(CSSOMString property);
  CSSOMString getPropertyPriority(CSSOMString property);
  [CEReactions] undefined setProperty(CSSOMString property, [LegacyNullToEmptyString] CSSOMString value, optional [LegacyNullToEmptyString] CSSOMString priority = "");
  [CEReactions] CSSOMString removeProperty(CSSOMString property);
  readonly attribute CSSRule? parentRule;
  [CEReactions] attribute [LegacyNullToEmptyString] CSSOMString cssFloat;
};

// CSSOM View : géométrie et défilement des éléments.
partial interface Element {
  DOMRectList getClientRects();
  [NewObject] DOMRect getBoundingClientRect();

  undefined scrollIntoView(optional (boolean or ScrollIntoViewOptions) arg = {});
  undefined scroll(optional ScrollToOptions options = {});
  undefined scrollTo(optional ScrollToOptions options = {});
  undefined scrollBy(optional ScrollToOptions options = {});
  attribute unrestricted double scrollTop;
  attribute unrestricted double scrollLeft;
  readonly attribute long scrollWidth;
  readonly attribute long scrollHeight;
  readonly attribute long clientTop;
  readonly attribute long clientLeft;
  readonly attribute long clientWidth;
  readonly attribute long clientHeight;
};

// DOM Parsing and Serialization.
partial interface Element {
  [CEReactions] attribute (TrustedHTML or [LegacyNullToEmptyString] DOMString) innerHTML;
  [CEReactions] attribute (TrustedHTML or [LegacyNullToEmptyString] DOMString) outerHTML;
  [CEReactions] undefined insertAdjacentHTML(DOMString position, (TrustedHTML or DOMString) string);
};

// Web Storage (https://html.spec.whatwg.org/multipage/webstorage.html).
[Exposed=Window]
interface Storage {
  readonly attribute unsigned long length;
  DOMString? key(unsigned long index);
  getter DOMString? getItem(DOMString key);
  setter undefined setItem(DOMString key, DOMString value);
  deleter undefined removeItem(DOMString key);
  undefined clear();
};
