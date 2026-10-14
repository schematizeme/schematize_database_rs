//! A CASCA de plataforma — o que todo app da casa repete, e nada de domínio.
//!
//! **O quê:** procedência do binário, resolução de executável, utilitários de caminho.
//!
//! **Onde:** `main` e os subcomandos.
//!
//! **Por que é fina (ADR-0026):** o que é plataforma de verdade (market parasita, procedência,
//! `home`/`resolve_bin`) vive em `schematize_plataforma`; aqui ficam só `desktop`/`icone` e a
//! cola com o `build.rs` deste repo. Nada de domínio entra nesta pasta (piso 6).

pub mod desktop;
pub mod icone;
pub mod procedencia;

/// `home`/`resolve_bin` vivem na lib compartilhada; o alias mantém `nucleo::bin::...` nos chamadores.
pub use schematize_plataforma::bin;
