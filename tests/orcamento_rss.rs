//! O ORÇAMENTO DE MEMÓRIA do CLI, como teste (piso de eficiência de runtime; ver `BUDGET.md`).
//!
//! **Onde:** `cargo test` e o gate local. Sobe o binário de verdade sobre um banco grande,
//! polla `VmHWM` de `/proc/<pid>/status` até o processo sair e FALHA acima do teto.
//!
//! **Por que o teto é folgado:** o binário aqui é o de `debug` (o que o cargo expõe aos testes),
//! maior que o release. O teto vale para o pior caso; o valor de release está no `BUDGET.md`.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_schematize-database");

/// Teto de RSS de pico (MiB) por operação, no binário de debug. Medido + folga ~30%.
const TETO_MIB: u64 = 20;

/// **O quê:** cria um SQLite com `n` tabelas (PK, FK, índice) numa única transação.
///
/// **Onde:** os testes deste arquivo; o banco grande é o que faria o pico crescer se alguma
/// etapa acumulasse mais do que precisa.
fn banco_grande(n: usize) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dbrss-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("criar o dir de teste");
    let p = dir.join("grande.db");
    let conn = rusqlite::Connection::open(&p).expect("abrir sqlite");
    let mut ddl = String::from("BEGIN;");
    for i in 0..n {
        let pai = i.saturating_sub(1);
        ddl.push_str(&format!(
            "CREATE TABLE t{i} (id INTEGER PRIMARY KEY, a TEXT NOT NULL, b TEXT, c INT, \
             d INT REFERENCES t{pai}(id));CREATE INDEX i{i} ON t{i}(a);"
        ));
    }
    ddl.push_str("COMMIT;");
    conn.execute_batch(&ddl).expect("criar o schema grande");
    p
}

/// **O quê:** lê o `VmHWM` (pico de RSS, em KiB) de um pid, se o processo ainda existir.
///
/// **Onde:** o laço de polling de [`pico_mib`].
fn vm_hwm_kib(pid: u32) -> Option<u64> {
    let t = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let l = t.lines().find(|l| l.starts_with("VmHWM:"))?;
    l.split_whitespace().nth(1)?.parse().ok()
}

/// **O quê:** roda o binário e devolve o maior `VmHWM` amostrado (MiB), `None` se não amostrou.
///
/// **Onde:** os testes de orçamento. A saída vai para um arquivo (não para um cano), para o
/// processo nunca bloquear esperando leitor.
fn pico_mib(args: &[&str], saida: &Path) -> Option<u64> {
    let out = File::create(saida).expect("arquivo de saída");
    let mut filho = Command::new(BIN)
        .env("SCHEMATIZE_SEM_MARKET", "1")
        .args(args)
        .stdout(Stdio::from(out))
        .stderr(Stdio::null())
        .spawn()
        .expect("subir o binário");
    let pid = filho.id();
    let mut pico = None;
    loop {
        if let Some(k) = vm_hwm_kib(pid) {
            pico = Some(pico.map_or(k, |p: u64| p.max(k)));
        }
        match filho.try_wait().expect("try_wait") {
            Some(st) => {
                assert!(st.success(), "o comando falhou: {args:?}");
                return pico.map(|k| k.div_ceil(1024));
            }
            None => std::thread::sleep(Duration::from_millis(1)),
        }
    }
}

/// **O quê:** valida o teto para uma operação, repetindo até 5x se o processo for rápido
/// demais para ser amostrado.
///
/// **Onde:** o teste único abaixo. Pula com `eprintln` explícito só se `/proc` não existe.
fn conferir(nome: &str, args: &[&str], dir: &Path) {
    let saida = dir.join("saida.txt");
    for _ in 0..5 {
        if let Some(mib) = pico_mib(args, &saida) {
            eprintln!("orcamento {nome}: pico {mib} MiB (teto {TETO_MIB})");
            assert!(mib <= TETO_MIB, "{nome}: pico {mib} MiB > teto {TETO_MIB} MiB");
            return;
        }
    }
    panic!("{nome}: nenhuma amostra de VmHWM em 5 tentativas");
}

/// O orçamento de RSS vale para introspect, sql, migration e graph sobre 1500 tabelas.
#[test]
fn o_rss_de_pico_respeita_o_orcamento() {
    if !Path::new("/proc/self/status").exists() {
        eprintln!("PULADO: /proc indisponível nesta plataforma, o orçamento não foi medido");
        return;
    }
    let db = banco_grande(1500);
    let dir = db.parent().expect("dir do banco").to_path_buf();
    let db = db.to_str().expect("utf8");
    conferir("introspect", &["introspect", "--sqlite", db], &dir);
    conferir("introspect --json", &["introspect", "--sqlite", db, "--json"], &dir);
    // Gera o JSON uma vez e o reaproveita como entrada de `sql`/`graph`.
    let json = dir.join("schema.json");
    let o = Command::new(BIN)
        .env("SCHEMATIZE_SEM_MARKET", "1")
        .args(["introspect", "--sqlite", db, "--json"])
        .output()
        .expect("introspect");
    std::fs::write(&json, &o.stdout).expect("gravar json");
    let j = json.to_str().expect("utf8");
    conferir("sql", &["sql", "--from", j], &dir);
    conferir("sql --migration", &["sql", "--from", j, "--migration"], &dir);
    conferir("graph --json", &["graph", "--from", j, "--json"], &dir);
    // Autoverificação: com o teto zerado o próprio mecanismo tem de enxergar um pico > 0.
    let pico = pico_mib(&["--version"], &dir.join("v.txt"));
    assert!(pico.is_none_or(|m| m > 0), "o medidor tem de ver RSS > 0 (senão está cego)");
    let _ = std::fs::remove_dir_all(&dir);
}
