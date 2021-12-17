# sml-rs

Smart Message Language (SML) parser written in Rust.

This library is still a work in progress, so expect that things might change in the future. The implementation doesn't cover the whole SML spec yet, but can successfully parse data transmissions sent by many smart meters.

- SML V1.04 Spec [[pdf]](https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publikationen/TechnischeRichtlinien/TR03109/TR-03109-1_Anlage_Feinspezifikation_Drahtgebundene_LMN-Schnittstelle_Teilb.pdf;jsessionid=F2323041EE7292926D80680DA407BA3F.internet082?__blob=publicationFile&v=1) [[archived]](https://web.archive.org/web/20211217153839/https://www.bsi.bund.de/SharedDocs/Downloads/DE/BSI/Publikationen/TechnischeRichtlinien/TR03109/TR-03109-1_Anlage_Feinspezifikation_Drahtgebundene_LMN-Schnittstelle_Teilb.pdf;jsessionid=F2323041EE7292926D80680DA407BA3F.internet082?__blob=publicationFile&v=1)
- Focus on correctness (conformance to the spec, no crashes), portability (no_std support) and performance (efficient parser implementation)
- Good test coverage