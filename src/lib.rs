//! **schematize-database** — o desenho do banco, antes da primeira linha de aplicação.
//!
//! **O quê:** modela o schema relacional, lê o banco que já existe (SQLite e Postgres), e gera
//! o SQL e a migration expand-contract.
//!
//! **Onde:** o binário `schematize-database` e a janela dele.
//!
//! ## Por que é um app, e não uma tela do hub (ADR-0018, fase E1)
//!
//! Nada aqui fala de skill, de overdev ou de instalação — as três coisas que definem o hub.
//! Enquanto morou no `schematize_cli_rs`, foi o candidato mais limpo do inventário de
//! extradição: dois arquivos no CLI, dois na janela, e uma fronteira que já era JSON.
//!
//! O piso 10 vale: este app sobe sozinho, e a ausência dele não impede o hub de bootar — a aba
//! do hub vira `TelaDelegada` e diz o que falta.

pub mod dominio;
pub mod nucleo;

pub use dominio::{Column, Edge, Fk, Index, Node, Schema, Table};
