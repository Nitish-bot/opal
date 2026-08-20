# Unicode statement handling across JavaScript and Solana

**Research date:** 2026-08-11
**Scope:** Preventing length and display mismatches between browser JavaScript, Anchor/Borsh instruction serialization, and Opal's fixed 280-byte statement field.

## Recommendation

Keep Opal's current exact-byte rule; Unicode normalization is not needed to make JavaScript display and size statements correctly.

1. Require a well-formed JavaScript string and reject isolated UTF-16 surrogates.
2. Preserve the submitted string without normalization.
3. Measure the protocol limit with `new TextEncoder().encode(statement).byteLength`, never `statement.length`.
4. Show the user the same string instance that is passed to the Anchor instruction and label the counter explicitly, for example `126 / 280 UTF-8 bytes`.
5. Onchain, accept the Anchor/Borsh `String`, apply the structural checks, use `statement.as_bytes().len()` for the 1–280-byte limit, and copy those bytes unchanged into the account.
6. When reading the account, select the bytes before the first zero-padding byte (or all 280 bytes when there is no zero), decode with fatal UTF-8 handling, and display that decoded value without normalization.

This completely solves the JavaScript length mismatch while retaining exact wallet-visible text. Canonically equivalent spellings can still produce distinct statement bytes, but Assertions are identified by `assertionId`, not by rendered text.

## Why JavaScript's normal length is the wrong unit

ECMAScript strings are sequences of 16-bit values, and their `length` is the number of those values. It is therefore a UTF-16 code-unit count, not a UTF-8 byte count or a count of user-perceived characters. ([ECMAScript String type](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-ecmascript-language-types-string-type))

The relevant units differ:

| Measurement                | JavaScript mechanism                               | Appropriate use                           |
| -------------------------- | -------------------------------------------------- | ----------------------------------------- |
| UTF-16 code units          | `statement.length`                                 | JavaScript indexing; not an Opal limit    |
| Unicode code points        | `[...statement].length`                            | Diagnostics; not storage size             |
| Extended grapheme clusters | `Intl.Segmenter(..., { granularity: "grapheme" })` | Optional user-facing character estimate   |
| UTF-8 bytes                | `new TextEncoder().encode(statement).byteLength`   | The authoritative 280-byte protocol limit |

Unicode defines grapheme clusters as an approximation of user-perceived characters, and ECMA-402 exposes grapheme segmentation through `Intl.Segmenter`. A grapheme can nevertheless contain several code points and bytes, so this count can only be informational. ([Unicode grapheme boundaries](https://www.unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries), [ECMA-402 `Intl.Segmenter`](https://tc39.es/ecma402/#segmenter-objects))

`TextEncoder` needs no encoding argument because it supports only UTF-8. Its `encode()` method returns the UTF-8 result as a `Uint8Array`, making `.byteLength` the exact unit used by Opal's fixed byte field. ([WHATWG `TextEncoder`](https://encoding.spec.whatwg.org/#interface-textencoder))

For example, the precomposed `Caf\u00E9` is five UTF-8 bytes, while the canonically equivalent decomposed `Cafe\u0301` is six. Both normally render as “Café.” The byte counter is allowed to differ because it describes storage, not visual width.

## Preventing a preview-to-transaction mismatch

An ECMAScript string can contain an isolated UTF-16 surrogate even though that is not a Unicode scalar value. `TextEncoder.encode()` accepts a Web IDL `USVString`; converting to that type replaces isolated surrogates with U+FFFD. Rejecting a string for which `statement.isWellFormed()` is false prevents that silent replacement. ([ECMAScript `isWellFormed`](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.iswellformed), [WHATWG scalar-value conversion](https://infra.spec.whatwg.org/#javascript-string-convert))

A future client can use this core sequence:

```ts
const encoder = new TextEncoder();

if (!statement.isWellFormed()) {
  throw new Error('INVALID_UNICODE');
}

const statementBytes = encoder.encode(statement);

if (statementBytes.byteLength === 0) {
  throw new Error('STATEMENT_EMPTY');
}

if (statementBytes.byteLength > 280) {
  throw new Error('STATEMENT_TOO_LONG');
}

// Apply the separate whitespace/control/line checks, preview `statement`,
// then pass this same value to the Anchor instruction builder.
```

Anchor uses Borsh as its default instruction serialization. Borsh represents a `String` as its UTF-8 bytes preceded by their `u32` length, while Rust's `String` is guaranteed to contain UTF-8. Consequently, for a well-formed JavaScript input, the browser's `TextEncoder` byte length and the program's `statement.as_bytes().len()` refer to the same encoding. ([Anchor serialization](https://docs.rs/anchor-lang/0.32.1/anchor_lang/trait.AnchorSerialize.html), [Borsh string specification](https://borsh.io/), [Rust `String`](https://doc.rust-lang.org/std/string/struct.String.html))

The client should not calculate bytes from one value and serialize another. Validation, byte counting, preview, and transaction construction should share a single immutable validated value. Account readers should use `TextDecoder("utf-8", { fatal: true })` rather than replacement decoding so corrupt bytes fail visibly; the Encoding Standard specifies that fatal decoding throws on an encoding error. ([WHATWG `TextDecoder`](https://encoding.spec.whatwg.org/#interface-textdecoder))

## Preserve exact UTF-8 versus require NFC

### Preserve exact UTF-8

Advantages:

- no hidden transformation between input, preview, signature, and storage;
- no Unicode normalization tables or version become part of onchain consensus;
- the `TextEncoder` rule fully solves the JavaScript byte-limit issue.

Consequence: canonically equivalent sequences can remain byte-distinct. This is not a storage or display malfunction; it is an accepted identity edge case because text is not the Assertion identifier.

### Require NFC

Unicode NFC maps canonically equivalent sequences to a common binary form where possible. JavaScript provides the standardized `statement.normalize("NFC")`; NFC is also the method's default. Unicode recommends NFC for web-content interoperability. ([Unicode Normalization Forms](https://www.unicode.org/reports/tr15/), [ECMAScript `normalize`](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.normalize))

If Opal adopted NFC, the safe order would be:

1. reject an ill-formed JavaScript string;
2. compute `normalized = statement.normalize("NFC")`;
3. clearly preview `normalized`, not the pre-normalized input;
4. measure and submit `TextEncoder().encode(normalized)`;
5. have the program reject any received string that is not NFC.

Client-only normalization is not a protocol rule because a custom client can bypass it. Onchain enforcement is technically plausible: Rust's standard `String` API does not perform normalization, but the `unicode-normalization` crate supplies NFC iteration and supports `no_std + alloc`. It has not been built or benchmarked in Opal. Adoption would therefore require a Solana-target prototype, compute and program-size measurement, a pinned crate/Unicode-data version, cross-language conformance vectors, and an onchain `received == NFC(received)` check. ([Rust `unicode-normalization` API](https://docs.rs/unicode-normalization/latest/unicode_normalization/trait.UnicodeNormalization.html), [`no_std + alloc` support](https://github.com/unicode-rs/unicode-normalization#no_std--alloc-support))

NFC still does not make JavaScript `length` equal UTF-8 byte length, so `TextEncoder` remains necessary. It also does not eliminate visually confusable characters such as a Cyrillic letter that resembles a Latin letter. Unicode treats confusable detection as a separate, imperfect mechanism and warns that confusable skeletons are not a storage normalization. ([Unicode Security Mechanisms](https://www.unicode.org/reports/tr39/#Confusable_Detection))

Compatibility normalization (`NFKC` or `NFKD`) is not suitable for immutable natural-language statements because Unicode warns that it can erase distinctions important to meaning. ([UAX #15 compatibility normalization](https://www.unicode.org/reports/tr15/#Compatibility_Equivalence_Figure))

## Decision implication

The reported “mismatch hassle” is a JavaScript measurement issue, not a reason by itself to normalize. `TextEncoder` plus well-formed-string rejection yields the exact onchain byte count while the browser continues to render the string normally. Opal should retain exact UTF-8 preservation unless product requirements separately decide that canonical-equivalent statement spellings must collapse to one byte representation; only that latter requirement justifies making NFC and its Unicode version part of the protocol.
