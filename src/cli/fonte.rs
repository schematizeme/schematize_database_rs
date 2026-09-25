//! De ONDE vem o schema, e o resumo humano dele.
//!
//! **Onde:** o despacho de `cli::executar`.

use database::dominio::{self, Schema};

/// **O quê:** resolve a fonte do schema — `--from <json>` | `--sqlite` | `--postgres`.
///
/// **Onde:** os três subcomandos.
///
/// **A ordem é a da especificidade:** um `--from` explícito ganha do banco, porque quem passou
/// um arquivo quer AQUELE schema (tipicamente um editado à mão ou salvo pela janela), e ler o
/// banco por cima dele descartaria a edição em silêncio.
///
/// **Sem fonte é ERRO, e o erro LISTA as três** — piso 4 e §37.48: mensagem acionável, que
/// ensina o que fazer em vez de dizer que a pessoa errou.
pub fn resolver(
    from: Option<String>,
    sqlite: Option<String>,
    postgres: Option<String>,
) -> Result<Schema, String> {
    if let Some(f) = from {
        let s = std::fs::read_to_string(&f).map_err(|e| format!("ler {f}: {e}"))?;
        return serde_json::from_str(&s).map_err(|e| format!("schema JSON inválido em {f}: {e}"));
    }
    if let Some(p) = sqlite {
        return dominio::introspect_sqlite(std::path::Path::new(&p));
    }
    if let Some(c) = postgres {
        return dominio::introspect_postgres(&c);
    }
    Err("informe a fonte do schema: --from <schema.json> | --sqlite <arquivo> | --postgres <conn>"
        .into())
}

/// **O quê:** o resumo humano de um schema — tabela a tabela, com os totais.
///
/// **Onde:** `introspect` sem `--json`.
pub fn resumo(schema: &Schema) {
    let mut cols = 0usize;
    let mut fks = 0usize;
    for t in &schema.tables {
        cols += t.columns.len();
        fks += t.fks.len();
        let pk: Vec<&str> = t.columns.iter().filter(|c| c.pk).map(|c| c.name.as_str()).collect();
        println!(
            "  {} — {} coluna(s), {} FK(s), {} índice(s){}",
            t.name,
            t.columns.len(),
            t.fks.len(),
            t.indexes.len(),
            if pk.is_empty() { String::new() } else { format!("; PK: {}", pk.join(", ")) }
        );
    }
    println!("total: {} tabela(s), {cols} coluna(s), {fks} FK(s).", schema.tables.len());
}
