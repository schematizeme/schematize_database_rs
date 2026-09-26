//! O que cada tela mostra — lido do `--json` do binário headless, nunca de saída humana.
//!
//! **O quê:** transforma os documentos de `introspect --json` e `graph --json` nas linhas que a
//! janela desenha.
//!
//! **Onde:** [`crate::gui`], que liga isto às propriedades do Slint.
//!
//! ## Funções PURAS, e a razão tem nome
//!
//! Tudo aqui entra texto e sai lista. O subprocesso fica em [`crate::gui::cli`]. Assim o
//! comportamento é afirmável com JSON inválido, truncado, hostil ou de outro idioma — sem
//! depender do que está instalado na máquina de quem roda a suíte.
//!
//! Este ecossistema já pagou duas vezes por não ter feito isso: a janela que casava **rótulo em
//! português** e devolvia vazio nos outros dezenove idiomas sem erro nenhum, e a pílula que saía
//! vazia contra um binário de outra versão.

use super::json::{ler, Json};

/// Uma coluna de tabela, já pronta para a tela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coluna {
    pub nome: String,
    pub tipo: String,
    /// `PK`, `UNIQUE`, `NOT NULL` — as marcas que o desenho do schema comunica.
    pub marcas: String,
}

/// Uma tabela, com o que ela tem de mostrar numa linha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tabela {
    pub nome: String,
    pub colunas: Vec<Coluna>,
    /// `pedidos.cliente_id -> clientes.id`, uma por linha.
    pub fks: Vec<String>,
    /// `idx_nome (col_a, col_b)` — com `UNIQUE` quando for.
    pub indices: Vec<String>,
}

/// Uma aresta do grafo de schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aresta {
    pub de: String,
    pub para: String,
    pub coluna: String,
}

/// **O quê:** o schema lido do `introspect --json`.
///
/// **Onde:** a aba Schema.
///
/// **`Err` e lista vazia são coisas diferentes:** um banco sem tabelas é `Ok(vec![])`, e a tela
/// diz "nenhuma tabela". Um documento ilegível é `Err`, e a tela mostra o erro. Confundir os
/// dois faria a janela afirmar "banco vazio" sobre um erro de leitura — o mesmo defeito que o
/// ADR-0015 registrou.
pub fn ler_schema(texto: &str) -> Result<Vec<Tabela>, String> {
    let doc = ler(texto)?;
    Ok(doc
        .arr("tables")
        .iter()
        .map(|t| Tabela {
            nome: t.str_ou_vazio("name"),
            colunas: t
                .arr("columns")
                .iter()
                .map(|c| Coluna {
                    nome: c.str_ou_vazio("name"),
                    tipo: c.str_ou_vazio("ty"),
                    marcas: marcas_da_coluna(c),
                })
                .collect(),
            fks: t
                .arr("fks")
                .iter()
                .map(|f| {
                    format!(
                        "{} -> {}.{}",
                        f.str_ou_vazio("column"),
                        f.str_ou_vazio("ref_table"),
                        f.str_ou_vazio("ref_column")
                    )
                })
                .collect(),
            indices: t.arr("indexes").iter().map(indice_em_texto).collect(),
        })
        .collect())
}

/// **O quê:** as marcas de uma coluna, na ordem em que importam ao ler um schema.
///
/// **Onde:** [`ler_schema`].
///
/// **PK primeiro:** é o que se procura primeiro ao bater o olho numa tabela. `NOT NULL` por
/// último porque é o mais comum — pôr o comum na frente afoga o que distingue.
fn marcas_da_coluna(c: &Json) -> String {
    let mut m: Vec<&str> = Vec::new();
    if c.bool("pk") == Some(true) {
        m.push("PK");
    }
    if c.bool("unique") == Some(true) {
        m.push("UNIQUE");
    }
    // `nullable: false` é o que vira NOT NULL. A ausência do campo NÃO conta como `false`:
    // campo que não veio é "não sei", e afirmar NOT NULL sobre isso seria inventar.
    if c.bool("nullable") == Some(false) {
        m.push("NOT NULL");
    }
    m.join(" · ")
}

fn indice_em_texto(i: &Json) -> String {
    let cols: Vec<&str> = i.arr("columns").iter().filter_map(|c| c.como_str()).collect();
    let unico = if i.bool("unique") == Some(true) { " UNIQUE" } else { "" };
    format!("{} ({}){}", i.str_ou_vazio("name"), cols.join(", "), unico)
}

/// **O quê:** as arestas do `graph --json`.
///
/// **Onde:** a aba Schema, na seção de relações.
pub fn ler_grafo(texto: &str) -> Result<Vec<Aresta>, String> {
    let doc = ler(texto)?;
    Ok(doc
        .arr("edges")
        .iter()
        .map(|e| Aresta {
            de: e.str_ou_vazio("from"),
            para: e.str_ou_vazio("to"),
            coluna: e.str_ou_vazio("label"),
        })
        .collect())
}

/// **O quê:** o resumo de um schema numa linha — o que a aba mostra no topo.
///
/// **Onde:** a aba Schema.
pub fn resumo(tabelas: &[Tabela]) -> String {
    let cols: usize = tabelas.iter().map(|t| t.colunas.len()).sum();
    let fks: usize = tabelas.iter().map(|t| t.fks.len()).sum();
    format!("{} tabela(s) · {cols} coluna(s) · {fks} FK(s)", tabelas.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"{"tables":[
      {"name":"clientes",
       "columns":[{"name":"id","ty":"INTEGER","pk":true,"nullable":true,"unique":false},
                  {"name":"email","ty":"TEXT","pk":false,"nullable":false,"unique":true}],
       "fks":[],"indexes":[]},
      {"name":"pedidos",
       "columns":[{"name":"id","ty":"INTEGER","pk":true,"nullable":true,"unique":false}],
       "fks":[{"column":"cliente_id","ref_table":"clientes","ref_column":"id"}],
       "indexes":[{"name":"idx_ped_cli","columns":["cliente_id"],"unique":false}]}
    ]}"#;

    #[test]
    fn le_tabelas_colunas_fks_e_indices() {
        let t = ler_schema(DOC).expect("lê");
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].nome, "clientes");
        assert_eq!(t[0].colunas[0].marcas, "PK");
        assert_eq!(t[0].colunas[1].marcas, "UNIQUE · NOT NULL");
        assert_eq!(t[1].fks, ["cliente_id -> clientes.id"]);
        assert_eq!(t[1].indices, ["idx_ped_cli (cliente_id)"]);
    }

    /// **Campo ausente é "não sei", e não `false`.** Afirmar NOT NULL sobre um campo que não
    /// veio seria inventar uma restrição que o banco não tem — e alguém escreveria a migration
    /// a partir disso.
    #[test]
    fn campo_ausente_nao_vira_afirmacao() {
        let t = ler_schema(r#"{"tables":[{"name":"t","columns":[{"name":"c","ty":"TEXT"}]}]}"#)
            .expect("lê");
        assert_eq!(t[0].colunas[0].marcas, "", "sem os campos, nenhuma marca é afirmada");
    }

    /// Índice único é marcado — é a diferença entre uma restrição e uma otimização.
    #[test]
    fn indice_unico_e_marcado() {
        let t = ler_schema(
            r#"{"tables":[{"name":"t","columns":[],"fks":[],
                "indexes":[{"name":"u","columns":["a","b"],"unique":true}]}]}"#,
        )
        .expect("lê");
        assert_eq!(t[0].indices, ["u (a, b) UNIQUE"]);
    }

    /// **Vazio e ERRO são estados diferentes.**
    #[test]
    fn vazio_e_erro_nao_se_confundem() {
        assert_eq!(ler_schema(r#"{"tables":[]}"#).expect("vazio é Ok"), vec![]);
        assert!(ler_schema("{ isto nao e json").is_err(), "ilegível é Err, não lista vazia");
        // Documento válido sem a chave: é Ok e vazio. JSON legítimo que não tem tabela.
        assert_eq!(ler_schema("{}").expect("objeto vazio"), vec![]);
    }

    /// Entrada hostil não panica — janela que morre ao abrir é pior que lista vazia.
    #[test]
    fn entrada_hostil_nao_panica() {
        let fundo = format!("{}{}", "[".repeat(3000), "]".repeat(3000));
        for lixo in [
            "",
            "null",
            "[]",
            "0",
            "\"txt\"",
            "\u{0}",
            &fundo,
            r#"{"tables":"nao e lista"}"#,
            r#"{"tables":[null]}"#,
            r#"{"tables":[{"name":42,"columns":7}]}"#,
        ] {
            let _ = ler_schema(lixo);
            let _ = ler_grafo(lixo);
        }
    }

    #[test]
    fn le_o_grafo() {
        let g = ler_grafo(
            r#"{"nodes":[],"edges":[{"from":"pedidos","to":"clientes","label":"cliente_id"}]}"#,
        )
        .expect("lê");
        assert_eq!(
            g,
            [Aresta { de: "pedidos".into(), para: "clientes".into(), coluna: "cliente_id".into() }]
        );
    }

    #[test]
    fn o_resumo_conta_o_que_importa() {
        let t = ler_schema(DOC).expect("lê");
        assert_eq!(resumo(&t), "2 tabela(s) · 3 coluna(s) · 1 FK(s)");
    }
}
