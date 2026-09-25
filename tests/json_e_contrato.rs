//! O `--json` é CONTRATO com outro repositório — e este arquivo é quem o trava.
//!
//! **Onde:** `cargo test`, e o CI.
//!
//! **Por que um teste de integração e não unitário:** o contrato é o que SAI do binário. Um
//! teste que chamasse a função de formatação provaria a função; este roda o comando e lê o
//! `stdout`, que é o que a janela vê de verdade.

use std::process::Command;

/// O caminho do binário deste crate, no perfil em que a suíte está rodando.
///
/// **Onde:** todos os testes daqui. `CARGO_BIN_EXE_` é o cargo dizendo onde o binário ficou —
/// montar o caminho à mão erraria entre `debug` e `release`.
const BIN: &str = env!("CARGO_BIN_EXE_schematize-database");

/// Cria um SQLite de teste e devolve o caminho.
///
/// **Onde:** os testes que precisam de um banco. O arquivo nasce num diretório temporário por
/// processo, então duas execuções em paralelo não disputam o mesmo caminho.
fn banco(nome: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("dbtest-{}-{nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("criar o dir de teste");
    let p = dir.join("t.db");
    let conn = rusqlite::Connection::open(&p).expect("abrir sqlite");
    conn.execute_batch(
        "CREATE TABLE clientes (id INTEGER PRIMARY KEY, email TEXT NOT NULL UNIQUE);
         CREATE TABLE pedidos (id INTEGER PRIMARY KEY,
                               cliente_id INTEGER NOT NULL REFERENCES clientes(id));
         CREATE INDEX idx_ped_cli ON pedidos(cliente_id);",
    )
    .expect("criar o schema");
    p
}

/// Roda o binário e devolve `(stdout, stderr, sucesso)`.
fn rodar(args: &[&str]) -> (String, String, bool) {
    let o = Command::new(BIN).args(args).output().expect("rodar o binário");
    (
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
        o.status.success(),
    )
}

/// **O CONTRATO do `introspect --json`.** Renomear um campo faz este teste mostrar a diferença.
#[test]
fn o_shape_do_introspect_e_contrato() {
    let p = banco("shape");
    let (out, err, ok) = rodar(&["introspect", "--sqlite", p.to_str().unwrap(), "--json"]);
    assert!(ok, "falhou: {err}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("tem de ser JSON válido");

    let topo: Vec<&str> = v.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(topo, ["tables"], "a raiz do documento é contrato");

    let t = &v["tables"][0];
    let chaves: Vec<&str> = t.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(chaves, ["columns", "fks", "indexes", "name"]);

    let c = &t["columns"][0];
    let chaves: Vec<&str> = c.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(chaves, ["name", "nullable", "pk", "ty", "unique"]);

    // A tabela `pedidos` é a que tem FK e índice.
    let ped = v["tables"]
        .as_array()
        .expect("lista")
        .iter()
        .find(|t| t["name"] == "pedidos")
        .expect("pedidos existe");
    let f = &ped["fks"][0];
    let chaves: Vec<&str> = f.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(chaves, ["column", "ref_column", "ref_table"]);
    let i = &ped["indexes"][0];
    let chaves: Vec<&str> = i.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(chaves, ["columns", "name", "unique"]);
}

/// **O CONTRATO do `graph --json`.**
#[test]
fn o_shape_do_grafo_e_contrato() {
    let p = banco("grafo");
    let (out, err, ok) = rodar(&["graph", "--sqlite", p.to_str().unwrap(), "--json"]);
    assert!(ok, "falhou: {err}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("JSON válido");
    let topo: Vec<&str> = v.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(topo, ["edges", "nodes"]);

    let n: Vec<&str> = v["nodes"][0].as_object().unwrap().keys().map(|s| s.as_str()).collect();
    assert_eq!(n, ["id", "loc"]);
    let e: Vec<&str> = v["edges"][0].as_object().unwrap().keys().map(|s| s.as_str()).collect();
    assert_eq!(e, ["from", "label", "to"]);

    // A aresta é a FK, e aponta do filho para o pai.
    assert_eq!(v["edges"][0]["from"], "pedidos");
    assert_eq!(v["edges"][0]["to"], "clientes");
    assert_eq!(v["edges"][0]["label"], "cliente_id");
}

/// **O documento é byte-a-byte IGUAL em qualquer idioma.**
///
/// O `--json` é decisão de máquina, e este projeto já pagou por ter parseado rótulo traduzido:
/// uma janela casava texto em português e devolvia vazio nos outros dezenove idiomas **sem erro
/// nenhum**. Aqui a regra é provada, não prometida.
#[test]
fn o_json_e_o_mesmo_em_qualquer_idioma() {
    let p = banco("idioma");
    let caminho = p.to_str().unwrap();
    let mut saidas = Vec::new();
    for lang in ["en_US.UTF-8", "pt_BR.UTF-8", "C", "ja_JP.UTF-8"] {
        let o = Command::new(BIN)
            .args(["introspect", "--sqlite", caminho, "--json"])
            // As SEIS variáveis: deixar uma de fora faz o teste passar por acidente numa
            // máquina cujo ambiente já estava no idioma esperado.
            .env_remove("LANGUAGE")
            .env_remove("LC_ALL")
            .env_remove("LC_MESSAGES")
            .env_remove("LC_CTYPE")
            .env_remove("SCHEMATIZE_LANG")
            .env("LANG", lang)
            .output()
            .expect("rodar");
        assert!(o.status.success(), "falhou em {lang}");
        saidas.push(String::from_utf8_lossy(&o.stdout).to_string());
    }
    for (i, s) in saidas.iter().enumerate().skip(1) {
        assert_eq!(&saidas[0], s, "o documento mudou com o idioma (índice {i})");
    }
}

/// **Nome de tabela vem do BANCO, e um banco aceita qualquer coisa.**
///
/// Aspas, barra e caractere de controle num nome de tabela têm de sair escapados — senão o
/// documento é JSON inválido e o outro lado falha com uma mensagem que fala do parser, nunca do
/// nome estranho que a causou.
#[test]
fn nome_hostil_de_tabela_nao_quebra_o_documento() {
    let dir = std::env::temp_dir().join(format!("dbtest-hostil-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("criar dir");
    let p = dir.join("t.db");
    let conn = rusqlite::Connection::open(&p).expect("abrir");
    // Aspas, barra invertida e TAB dentro do identificador. SQLite aceita com aspas duplas.
    conn.execute_batch(
        "CREATE TABLE \"tem\"\"aspas\" (id INTEGER PRIMARY KEY);
         CREATE TABLE \"tem\\barra\" (id INTEGER PRIMARY KEY);
         CREATE TABLE \"tem\tTAB\" (id INTEGER PRIMARY KEY);",
    )
    .expect("criar tabelas hostis");

    let (out, err, ok) = rodar(&["introspect", "--sqlite", p.to_str().unwrap(), "--json"]);
    assert!(ok, "falhou: {err}");
    let v: serde_json::Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("documento inválido: {e}\n{out}"));
    let nomes: Vec<String> = v["tables"]
        .as_array()
        .expect("lista")
        .iter()
        .map(|t| t["name"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(nomes.iter().any(|n| n.contains('"')), "a aspa tem de voltar como aspa: {nomes:?}");
    assert!(nomes.iter().any(|n| n.contains('\\')), "{nomes:?}");
    assert!(nomes.iter().any(|n| n.contains('\t')), "{nomes:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **Sem fonte é erro ACIONÁVEL, e ele lista as três.**
///
/// §37.48: mensagem que ensina o que fazer, em vez de dizer que a pessoa errou.
#[test]
fn sem_fonte_o_erro_diz_as_tres_opcoes() {
    let (_, err, ok) = rodar(&["sql"]);
    assert!(!ok, "sem fonte tem de sair 1");
    for pedaco in ["--from", "--sqlite", "--postgres"] {
        assert!(err.contains(pedaco), "o erro tem de listar `{pedaco}`: {err}");
    }
}

/// Arquivo que não existe, e JSON inválido em `--from`: erro claro, nunca panic.
#[test]
fn entrada_ruim_da_erro_e_nao_panic() {
    let (_, err, ok) = rodar(&["sql", "--sqlite", "/nao/existe/nunca.db"]);
    assert!(!ok);
    assert!(!err.contains("panicked"), "panicou: {err}");

    let dir = std::env::temp_dir().join(format!("dbtest-ruim-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let f = dir.join("lixo.json");
    std::fs::write(&f, "{ isto nao e json").expect("gravar");
    let (_, err, ok) = rodar(&["sql", "--from", f.to_str().unwrap()]);
    assert!(!ok);
    assert!(
        err.contains("inválido") || err.contains("JSON"),
        "o erro tem de nomear o problema: {err}"
    );
    assert!(!err.contains("panicked"), "panicou: {err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **A seta do grafo é ASCII** — contrato com o parser do app (C3).
#[test]
fn a_seta_do_grafo_e_ascii() {
    let p = banco("seta");
    let (out, _, ok) = rodar(&["graph", "--sqlite", p.to_str().unwrap()]);
    assert!(ok);
    assert!(out.contains(" -> "), "a aresta sai em ASCII: {out}");
    assert!(!out.contains('\u{2192}'), "seta unicode quebra o parser do app: {out}");
}

/// **A versão diz de qual COMMIT o binário é** — o estado que era indetectável antes.
#[test]
fn a_versao_traz_a_procedencia() {
    let (out, _, ok) = rodar(&["--version"]);
    assert!(ok);
    assert!(out.contains('('), "sem procedência: {out}");
    assert!(
        out.contains("fonte sem git") || out.chars().filter(|c| c.is_ascii_hexdigit()).count() >= 7,
        "{out}"
    );
}
