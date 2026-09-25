//! A SAÍDA DE MÁQUINA (`--json`) — o contrato com a janela e com o hub.
//!
//! **Onde:** `introspect --json` e `graph --json`.
//!
//! ## Escrito à MÃO, sem `derive` — e a razão é o leitor
//!
//! Este documento é lido por código que vive em **outro repositório** (a janela deste app, e o
//! hub quando delegar a aba). Com `derive`, renomear um campo do `Schema` muda o contrato sem
//! aparecer em lugar nenhum do diff — e o outro lado descobre isso em produção, lendo um campo
//! que sumiu. Escrevendo à mão, o contrato TEM de ser editado para mudar, e a edição aparece.
//!
//! É a mesma regra do `saidajson.rs` do optimizer e do deployer, pela mesma razão. (O oposto
//! vale para estado interno lido por `serde` dos dois lados — ver a fila de quiz do overdev.)
//!
//! ## As chaves NUNCA são traduzidas
//!
//! Chave de JSON é decisão de máquina. Este projeto já pagou por ter parseado rótulo em
//! português numa janela, que devolvia vazio nos outros dezenove idiomas **sem erro nenhum**.
//! Aqui não há uma única string de prosa — só nomes de tabela e de coluna, que vêm do banco.

use database::dominio::{Edge, Node, Schema};

/// **O quê:** escapa uma string para dentro de JSON.
///
/// **Onde:** todo lugar deste arquivo. Cobre as aspas, a barra, os controles obrigatórios e o
/// resto de `\u{0000}..\u{001f}` — nome de tabela vem do BANCO, e um banco pode ter qualquer
/// coisa ali. Um escape incompleto produz JSON que o outro lado recusa, e a mensagem de erro
/// fala do parser, nunca do nome estranho que a causou.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// `Option<&String>` → `"texto"` ou `null`.
///
/// **`null` e `""` são coisas DIFERENTES**, e o contrato preserva a diferença: um default que
/// não existe não é um default vazio.
fn opt(v: Option<&String>) -> String {
    match v {
        Some(s) => format!("\"{}\"", esc(s)),
        None => "null".into(),
    }
}

/// **O quê:** o schema inteiro como documento de máquina.
///
/// **Onde:** `introspect --json`.
pub fn schema(s: &Schema) -> String {
    let tabelas: Vec<String> = s
        .tables
        .iter()
        .map(|t| {
            let cols: Vec<String> = t
                .columns
                .iter()
                .map(|c| {
                    format!(
                        "{{\"name\":\"{}\",\"ty\":\"{}\",\"pk\":{},\"nullable\":{},\"unique\":{}}}",
                        esc(&c.name),
                        esc(&c.ty),
                        c.pk,
                        c.nullable,
                        c.unique
                    )
                })
                .collect();
            let fks: Vec<String> = t
                .fks
                .iter()
                .map(|f| {
                    format!(
                        "{{\"column\":\"{}\",\"ref_table\":\"{}\",\"ref_column\":\"{}\"}}",
                        esc(&f.column),
                        esc(&f.ref_table),
                        esc(&f.ref_column)
                    )
                })
                .collect();
            let idx: Vec<String> = t
                .indexes
                .iter()
                .map(|i| {
                    let colunas: Vec<String> =
                        i.columns.iter().map(|c| format!("\"{}\"", esc(c))).collect();
                    format!(
                        "{{\"name\":\"{}\",\"columns\":[{}],\"unique\":{}}}",
                        esc(&i.name),
                        colunas.join(","),
                        i.unique
                    )
                })
                .collect();
            format!(
                "{{\"name\":\"{}\",\"columns\":[{}],\"fks\":[{}],\"indexes\":[{}]}}",
                esc(&t.name),
                cols.join(","),
                fks.join(","),
                idx.join(",")
            )
        })
        .collect();
    format!("{{\"tables\":[{}]}}", tabelas.join(","))
}

/// **O quê:** o grafo do schema como documento de máquina.
///
/// **Onde:** `graph --json`. Tabela = nó, FK = aresta — o mesmo formato que o hub desenha.
pub fn grafo(nodes: &[Node], edges: &[Edge]) -> String {
    let ns: Vec<String> = nodes
        .iter()
        .map(|n| format!("{{\"id\":\"{}\",\"loc\":{}}}", esc(&n.id), opt(n.loc.as_ref())))
        .collect();
    let es: Vec<String> = edges
        .iter()
        .map(|e| {
            format!(
                "{{\"from\":\"{}\",\"to\":\"{}\",\"label\":{}}}",
                esc(&e.from),
                esc(&e.to),
                opt(e.label.as_ref())
            )
        })
        .collect();
    format!("{{\"nodes\":[{}],\"edges\":[{}]}}", ns.join(","), es.join(","))
}
