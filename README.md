# sml-rs

[![crates-io](https://img.shields.io/crates/v/sml-rs.svg?style=flat-square)](https://crates.io/crates/sml-rs)
[![api-docs](https://img.shields.io/docsrs/sml-rs.svg?style=flat-square)](https://docs.rs/sml-rs)
![CI](https://img.shields.io/github/workflow/status/fkohlgrueber/sml-rs/CI?label=CI&style=flat-square)
![Maintenance](https://img.shields.io/maintenance/yes/2022?style=flat-square)
[![LOC](https://img.shields.io/tokei/lines/github/fkohlgrueber/sml-rs?style=flat-square)](https://docs.rs/sml-rs)


Smart Message Language (SML) parser written in Rust.


`sml-rs` is an early work-in-progress and many features aren't implemented yet. See section "Implementation Status" for details.

## Spec

- SML V1.04 Spec [[pdf]](https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publikationen/TechnischeRichtlinien/TR03109/TR-03109-1_Anlage_Feinspezifikation_Drahtgebundene_LMN-Schnittstelle_Teilb.pdf;jsessionid=F2323041EE7292926D80680DA407BA3F.internet082?__blob=publicationFile&v=1) [[archive.org]](https://web.archive.org/web/20211217153839/https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publikationen/TechnischeRichtlinien/TR03109/TR-03109-1_Anlage_Feinspezifikation_Drahtgebundene_LMN-Schnittstelle_Teilb.pdf;jsessionid=F2323041EE7292926D80680DA407BA3F.internet082?__blob=publicationFile&v=1)


## Implementation status

- [ ] Transport v1
  - [x] Encode
  - [ ] Encode streaming
  - [ ] Decode
- [ ] Parsing
  - [ ] ...


#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>