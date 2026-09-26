//! A borda de LINHA DE COMANDO — o despacho e o que cada subcomando imprime.
//!
//! **Onde:** `main`. O domínio está em [`database::dominio`]; aqui é só entrada e saída.

pub mod args;
pub mod desktop;
pub mod fonte;
pub mod saidajson;

use args::Cmd;

/// **O quê:** despacha o subcomando.
///
/// **Onde:** `main`. Devolve `Result` em vez de imprimir e sair aqui dentro, porque o código de
/// saída é parte do contrato de quem encadeia este comando num script.
pub fn executar(cmd: Cmd) -> Result<(), String> {
    match cmd {
        Cmd::Introspect { sqlite, postgres, json } => {
            let schema = fonte::resolver(None, sqlite, postgres)?;
            if json {
                println!("{}", saidajson::schema(&schema));
                return Ok(());
            }
            println!("Schema ({} tabela(s)):", schema.tables.len());
            fonte::resumo(&schema);
            Ok(())
        }
        Cmd::Sql { from, sqlite, postgres, migration } => {
            let schema = fonte::resolver(from, sqlite, postgres)?;
            if migration {
                print!("{}", database::dominio::to_migration(&schema));
            } else {
                print!("{}", database::dominio::to_sql(&schema));
            }
            Ok(())
        }
        Cmd::Graph { from, sqlite, postgres, json } => {
            let schema = fonte::resolver(from, sqlite, postgres)?;
            let (nodes, edges) = database::dominio::to_graph(&schema);
            if json {
                println!("{}", saidajson::grafo(&nodes, &edges));
                return Ok(());
            }
            println!("nós ({}):", nodes.len());
            for n in &nodes {
                println!("  {}", n.id);
            }
            println!("arestas ({}):", edges.len());
            for e in &edges {
                // Seta em ASCII, como manda o C3: o parser do app lê `A -> B`, e um `→` quebra
                // a leitura. A saída daqui alimenta o grafo do hub.
                match &e.label {
                    Some(l) => println!("  {} -> {} ({l})", e.from, e.to),
                    None => println!("  {} -> {}", e.from, e.to),
                }
            }
            Ok(())
        }
        Cmd::Desktop { install, remover } => desktop::executar(install, remover),
    }
}
