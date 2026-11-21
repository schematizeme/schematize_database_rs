# BUDGET — orçamento de memória do `schematize-database`

Piso de eficiência de runtime da casa: orçamento declarado, medido no gate local, falha acima do teto.
Medido em 2026-11-21, Linux x86_64, binário release instalado (v0.1.5) e binário debug dos testes.
Carga: SQLite com 3000 tabelas (release) / 1500 tabelas (teste), cada uma com PK, FK e índice.

| Operação | RSS pico alvo | Medido antes | Medido depois | Como medir |
|---|---|---|---|---|
| `--version` (idle) | 16 MiB | 8,6 MiB | 8,6 MiB (sem mudança) | `wait4` -> `ru_maxrss` do filho |
| `introspect --sqlite` | 24 MiB | 12,7 MiB | 12,7 MiB | idem |
| `introspect --sqlite --json` | 24 MiB | 16,1 MiB | 16,1 MiB | idem |
| `sql --from` | 24 MiB | 9,9 MiB | 9,9 MiB | idem |
| `sql --from --migration` | 24 MiB | 9,9 MiB | 9,9 MiB | idem |
| `graph --from --json` | 24 MiB | 10,4 MiB | 10,4 MiB | idem |

Antes = depois: o diagnóstico não achou o que cortar com segurança (ver abaixo); nenhum código de produção mudou.

## Teste que faz valer

`tests/orcamento_rss.rs` sobe o binário (debug, o que o cargo expõe) sobre 1500 tabelas, polla `VmHWM` de
`/proc/<pid>/status` até a saída e falha acima de **20 MiB** (medido debug 9-13 MiB + folga). Pula com
`eprintln` explícito só se `/proc` não existe. `cargo test --test orcamento_rss -- --nocapture` mostra os picos.

## Tetos e justificativa

- CLI: teto 24 MiB, muito abaixo do limite da casa (256 MiB para CLI/serviço).
- GUI (`schematize-database-gui`, 20,5 MB de binário): **não medida, precisa de display**. Teto herdado da casa
  (400 MiB) até haver medição; o backend Slint/femtovg/winit é quem domina o RSS, não o código deste repo.

## Diagnóstico (por que não houve corte)

- CLI puro (`--no-default-features`) tem 58 pacotes; com GUI, 476 (toda a árvore do Slint, inevitável enquanto
  a janela morar no mesmo repo, ADR-0020; sem `gui`, o Slint nem entra no grafo).
- Todas as dependências diretas são usadas (clap derive, serde/serde_json em `dominio`/`fonte`, png no ícone,
  rusqlite). `rusqlite` já é `default-features = false`; Slint já é minimal (sem skia). Postgres lido por `psql`
  em vez de driver (evita TLS e ~40 crates).
- Sem leitura de arquivo grande, sem canal/cache sem limite, sem threads extras: o pico cresce ~linearmente e
  pouco com o nº de tabelas (3000 tabelas = 16 MiB).
- Candidatos descartados por não compensarem a mudança de comportamento: `clap` sem `color`/`suggestions`
  (altera `--help`/erros), `png` sem feature padrão.
