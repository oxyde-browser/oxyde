# Web Platform Tests

Produit par `cargo run --release -p oxyde_wpt -- --publier encoding`, sur la suite
`web-platform-tests/wpt` déposée dans `vendor/wpt`. Seuls les tests `testharness.js`
destinés à une fenêtre sont exécutés ; un fichier est « entièrement réussi » quand le
harnais va au bout et que tous ses sous-tests passent.

**Sous-tests réussis : 11559 / 247546 (4,7 %)** — fichiers entièrement réussis : 18 / 153 (11,8 %).

Le taux en sous-tests surestime la conformité : un fichier qui plante ou que le harnais
abandonne ne rapporte aucun sous-test, donc n'en fait échouer aucun. Le taux en fichiers
est le plus honnête des deux.

| Dossier | Fichiers réussis | Sous-tests réussis |
|---|---:|---:|
| `encoding` | 14 / 36 | 11358 / 12008 (94,6 %) |
| `encoding/legacy-mb-japanese/euc-jp` | 0 / 14 | 4 / 36441 (0,0 %) |
| `encoding/legacy-mb-japanese/iso-2022-jp` | 0 / 13 | 0 / 43642 (0,0 %) |
| `encoding/legacy-mb-japanese/shift_jis` | 0 / 24 | 20 / 132551 (0,0 %) |
| `encoding/legacy-mb-korean/euc-kr` | 0 / 26 | 4 / 6246 (0,1 %) |
| `encoding/legacy-mb-schinese/gb18030` | 0 / 2 | 1 / 529 (0,2 %) |
| `encoding/legacy-mb-schinese/gbk` | 0 / 2 | 1 / 131 (0,8 %) |
| `encoding/legacy-mb-tchinese/big5` | 1 / 23 | 126 / 15874 (0,8 %) |
| `encoding/streams` | 3 / 13 | 45 / 124 (36,3 %) |

## Fichiers non entièrement réussis

| Fichier | Sous-tests | Cause |
|---|---:|---|
| `encoding/big5-encoder.html` | 1 / 13 |  |
| `encoding/encodeInto-tail-unmodified.any.js` | 0 / 0 | boucle sans fin (processus tué) |
| `encoding/encodeInto.any.js` | 43 / 111 |  |
| `encoding/idlharness.any.js` | 0 / 1 |  |
| `encoding/iso-2022-jp-decoder.any.js` | 0 / 38 |  |
| `encoding/iso-2022-jp-encoder.html` | 1 / 12 |  |
| `encoding/json-document-utf8.html` | 0 / 5 |  |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-decode-cseucpkdfmtjapanese.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-japanese/euc-jp/eucjp-decode-errors.html` | 4 / 11 |  |
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
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-decode-csiso2022jp.html` | 0 / 7329 |  |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-decode-errors.html` | 0 / 8 |  |
| `encoding/legacy-mb-japanese/iso-2022-jp/iso2022jp-decode.html` | 0 / 7329 |  |
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
| `encoding/legacy-mb-japanese/shift_jis/sjis-decode-csshiftjis.html` | 2 / 7393 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-decode-errors.html` | 4 / 10 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-decode-ms932.html` | 2 / 7393 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-decode-ms_kanji.html` | 2 / 7393 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-decode-shift-jis.html` | 2 / 7393 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-decode-sjis.html` | 2 / 7393 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-decode-windows-31j.html` | 2 / 7393 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-decode-x-sjis.html` | 2 / 7393 |  |
| `encoding/legacy-mb-japanese/shift_jis/sjis-decode.html` | 2 / 7393 |  |
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
| `encoding/legacy-mb-korean/euc-kr/euckr-decode-errors.html` | 4 / 8 |  |
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
| `encoding/legacy-mb-schinese/gb18030/gb18030-decoder.any.js` | 0 / 275 |  |
| `encoding/legacy-mb-schinese/gb18030/gb18030-encoder.html` | 1 / 254 |  |
| `encoding/legacy-mb-schinese/gbk/gbk-decoder.any.js` | 0 / 82 |  |
| `encoding/legacy-mb-schinese/gbk/gbk-encoder.html` | 1 / 49 |  |
| `encoding/legacy-mb-tchinese/big5/big5-decode-big5-hkscs.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-decode-cn-big5.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-decode-csbig5.html` | 0 / 0 | le harnais n'a pas terminé |
| `encoding/legacy-mb-tchinese/big5/big5-decode-errors.html` | 3 / 8 |  |
| `encoding/legacy-mb-tchinese/big5/big5-decode-extra.html` | 0 / 2472 |  |
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
| `encoding/remove-only-one-bom.html` | 0 / 3 |  |
| `encoding/replacement-encodings.any.js` | 0 / 12 |  |
| `encoding/single-byte-decoder-iframe.window.js` | 0 / 168 |  |
| `encoding/single-byte-decoder.any.js` | 168 / 336 | harnais : délai du harnais dépassé |
| `encoding/sniffing.html` | 0 / 1 |  |
| `encoding/streams/backpressure.any.js` | 0 / 4 |  |
| `encoding/streams/decode-attributes.any.js` | 8 / 28 |  |
| `encoding/streams/decode-bad-chunks.any.js` | 0 / 5 | harnais : délai du harnais dépassé |
| `encoding/streams/decode-ignore-bom.any.js` | 10 / 12 |  |
| `encoding/streams/decode-incomplete-input.any.js` | 1 / 2 | harnais : délai du harnais dépassé |
| `encoding/streams/decode-non-utf8.any.js` | 4 / 26 | harnais : Unhandled rejection: TextDecoder : suite d'octets invalide en UTF-16BE |
| `encoding/streams/decode-split-character.any.js` | 5 / 11 |  |
| `encoding/streams/encode-bad-chunks.any.js` | 0 / 6 | harnais : délai du harnais dépassé |
| `encoding/streams/encode-utf8.any.js` | 6 / 19 |  |
| `encoding/streams/realms.window.js` | 0 / 0 | harnais : délai du harnais dépassé |
| `encoding/textdecoder-arguments.any.js` | 0 / 4 |  |
| `encoding/textdecoder-copy.any.js` | 0 / 2 |  |
| `encoding/textdecoder-eof.any.js` | 0 / 2 |  |
| `encoding/textdecoder-labels.any.js` | 184 / 222 |  |
| `encoding/textdecoder-mistakes.any.js` | 47 / 108 |  |
| `encoding/textdecoder-streaming.any.js` | 10 / 32 |  |
| `encoding/textencoder-constructor-non-utf.any.js` | 72 / 79 |  |
| `encoding/unsupported-encodings.any.js` | 0 / 14 |  |
| `encoding/utf-32-from-win1252.html` | 0 / 8 |  |
| `encoding/utf-32.html` | 3 / 8 |  |
