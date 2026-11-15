# Web Platform Tests

Produit par `cargo run --release -p oxyde_wpt -- --publier encoding`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Seuls les tests `testharness.js`
destinés à une fenêtre sont exécutés ; un fichier est « entièrement réussi » quand le
harnais va au bout et que tous ses sous-tests passent.

**Sous-tests réussis : 88370 / 247546 (35,7 %)** — fichiers entièrement réussis : 44 / 153 (28,8 %).

Le taux en sous-tests surestime la conformité : un fichier qui plante ou que le harnais
abandonne ne rapporte aucun sous-test, donc n'en fait échouer aucun. Le taux en fichiers
est le plus honnête des deux.

| Dossier | Fichiers réussis | Sous-tests réussis |
|---|---:|---:|
| `encoding` | 19 / 36 | 11515 / 12008 (95,9 %) |
| `encoding/legacy-mb-japanese/euc-jp` | 1 / 14 | 11 / 36441 (0,0 %) |
| `encoding/legacy-mb-japanese/iso-2022-jp` | 3 / 13 | 14666 / 43642 (33,6 %) |
| `encoding/legacy-mb-japanese/shift_jis` | 9 / 24 | 59154 / 132551 (44,6 %) |
| `encoding/legacy-mb-korean/euc-kr` | 1 / 26 | 8 / 6246 (0,1 %) |
| `encoding/legacy-mb-schinese/gb18030` | 1 / 2 | 276 / 529 (52,2 %) |
| `encoding/legacy-mb-schinese/gbk` | 1 / 2 | 83 / 131 (63,4 %) |
| `encoding/legacy-mb-tchinese/big5` | 3 / 23 | 2603 / 15874 (16,4 %) |
| `encoding/streams` | 6 / 13 | 54 / 124 (43,5 %) |

## Fichiers non entièrement réussis

| Fichier | Sous-tests | Cause |
|---|---:|---|
| `encoding/big5-encoder.html` | 1 / 13 |  |
| `encoding/encodeInto-tail-unmodified.any.js` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/encodeInto.any.js` | 43 / 111 |  |
| `encoding/idlharness.any.js` | 0 / 1 |  |
| `encoding/iso-2022-jp-encoder.html` | 1 / 12 |  |
| `encoding/json-document-utf8.html` | 0 / 5 |  |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-decode-cseucpkdfmtjapanese.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-decode-x-euc-jp.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-decode.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-form-cseucpkdfmtjapanese.html` | 0 / 7392 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-form-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-form-errors-hangul.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-form-errors-misc.html` | 0 / 3431 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-form-x-euc-jp.html` | 0 / 7392 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-form.html` | 0 / 7392 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-href-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-href-errors-hangul.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-href-errors-misc.html` | 0 / 3431 |  |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-encode-href.html` | 0 / 7392 |  |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-form-csiso2022jp.html` | 0 / 7329 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-form-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-form-errors-hangul.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-form-errors-misc.html` | 0 / 3431 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-form-errors-stateful.html` | 0 / 1 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-form.html` | 0 / 7392 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-href-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-href-errors-hangul.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-href-errors-misc.html` | 0 / 3431 |  |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-encode-href.html` | 0 / 7392 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-csshiftjis.html` | 0 / 7393 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-errors-hangul.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-errors-misc.html` | 0 / 3430 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-ms932.html` | 0 / 7393 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-ms_kanji.html` | 0 / 7393 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-shift-jis.html` | 0 / 7393 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-sjis.html` | 0 / 7393 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-windows-31j.html` | 0 / 7393 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form-x-sjis.html` | 0 / 7393 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-form.html` | 0 / 7393 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-href-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-href-errors-hangul.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-href-errors-misc.html` | 0 / 3430 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-encode-href.html` | 0 / 7393 |  |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-cseuckr.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-csksc56011987.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-iso-ir-149.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-korean.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-ks_c_5601-1987.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-ks_c_5601-1989.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-ksc5601.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-ksc_5601.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-windows-949.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-decode.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-cseuckr.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-csksc56011987.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-errors-misc.html` | 0 / 3119 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-iso-ir-149.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-korean.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-ks_c_5601-1987.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-ks_c_5601-1989.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-ksc5601.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-ksc_5601.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form-windows-949.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-form.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-href-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-href-errors-misc.html` | 0 / 3119 |  |
| `encoding/legacy-mb-korean/euc-kr/euckr-encode-href.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-schinese/gb18030/gb18030-encoder.html` | 1 / 254 |  |
| `encoding/legacy-mb-schinese/gbk/gbk-encoder.html` | 1 / 49 |  |
| `encoding/legacy-mb-tchinese/big5/big5-decode-big5-hkscs.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-decode-cn-big5.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-decode-csbig5.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-decode-x-x-big5.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-decode.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-big5-hkscs.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-cn-big5.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-csbig5.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-errors-extBa.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-errors-extBb.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-errors-hangul.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-errors-misc.html` | 0 / 3436 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-errors-pua.html` | 0 / 6399 | harnais : délai du harnais dépassé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form-x-x-big5.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-form.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-href-errors-han.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-href-errors-hangul.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-encode-href-errors-misc.html` | 0 / 3436 |  |
| `encoding/legacy-mb-tchinese/big5/big5-encode-href.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/replacement-encodings.any.js` | 0 / 12 |  |
| `encoding/single-byte-decoder-iframe.window.js` | 0 / 168 |  |
| `encoding/single-byte-decoder.any.js` | 168 / 336 | harnais : délai du harnais dépassé |
| `encoding/sniffing.html` | 0 / 1 |  |
| `encoding/streams/backpressure.any.js` | 0 / 4 |  |
| `encoding/streams/decode-attributes.any.js` | 8 / 28 |  |
| `encoding/streams/decode-bad-chunks.any.js` | 0 / 5 | harnais : délai du harnais dépassé |
| `encoding/streams/decode-non-utf8.any.js` | 4 / 26 | harnais : Unhandled rejection: TextDecoder : suite d'octets invalide en UTF-16BE |
| `encoding/streams/encode-bad-chunks.any.js` | 0 / 6 | harnais : délai du harnais dépassé |
| `encoding/streams/encode-utf8.any.js` | 6 / 19 |  |
| `encoding/streams/realms.window.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `encoding/textdecoder-arguments.any.js` | 3 / 4 |  |
| `encoding/textdecoder-copy.any.js` | 1 / 2 |  |
| `encoding/textdecoder-mistakes.any.js` | 106 / 108 |  |
| `encoding/textdecoder-streaming.any.js` | 16 / 32 |  |
| `encoding/unsupported-encodings.any.js` | 0 / 14 |  |
| `encoding/utf-32-from-win1252.html` | 0 / 8 |  |
| `encoding/utf-32.html` | 3 / 8 |  |
