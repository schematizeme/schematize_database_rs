//! A superfície da CLI.
//!
//! **Onde:** `main`, na raiz da árvore do clap.
//!
//! **Os nomes vieram do `schematize db <sub>` do hub, de propósito** (E1 do ADR-0018): quem já
//! digitava `db introspect --sqlite …` digita `schematize-database introspect --sqlite …`. Mudar
//! a forma do comando no MESMO dia em que ele muda de dono cobraria duas mudanças de quem usa.

use clap::{Parser, Subcommand};

/// `schematize-database` — modela o schema, lê o banco, gera o SQL.
#[derive(Parser)]
#[command(
    name = "schematize-database",
    // `version = <fn>` e nao `version` puro: o numero sozinho nao distingue dois binarios com o
    // mesmo `Cargo.toml` e comportamento diferente. Ver `nucleo/procedencia.rs`.
    version = database::nucleo::procedencia::rotulo_versao(),
    about = "schematize database — model the schema, read the database, emit the SQL",
    long_about = "Model a relational schema, introspect an existing database (SQLite or \
                  Postgres), and emit CREATE/ALTER/INDEX or an expand-contract migration.\n\
                  Works standalone; the schematize hub delegates its Database tab to it."
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Read an existing database and print its schema.
    Introspect {
        /// Path to a SQLite file.
        #[arg(long)]
        sqlite: Option<String>,
        /// Postgres connection string (read through `psql` in PATH).
        #[arg(long)]
        postgres: Option<String>,
        /// Machine-readable output (the window reads this).
        #[arg(long)]
        json: bool,
    },
    /// Emit SQL (CREATE/ALTER/INDEX) — or an expand-contract migration with --migration.
    Sql {
        /// Read the schema from a JSON file instead of a database.
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        sqlite: Option<String>,
        #[arg(long)]
        postgres: Option<String>,
        /// Emit an expand-contract migration instead of the plain CREATE.
        #[arg(long)]
        migration: bool,
    },
    /// Print the schema graph: table = node, FK = edge.
    Graph {
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        sqlite: Option<String>,
        #[arg(long)]
        postgres: Option<String>,
        /// Machine-readable output (the window reads this).
        #[arg(long)]
        json: bool,
    },
    /// Desktop integration: put this app in the applications menu, or take it out.
    ///
    /// **O app instala a PRÓPRIA integração** (ADR-0018): quem baixa o binário do release ou
    /// compila do fonte tem de conseguir o ícone também, sem passar pelo hub.
    Desktop {
        /// Write the icon and the .desktop entry.
        #[arg(long)]
        install: bool,
        /// Remove the .desktop entry (the icons stay — they are inert and shared).
        #[arg(long)]
        remover: bool,
    },
}
