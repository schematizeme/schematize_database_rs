//! **schematize-database-gui** — a janela do `schematize-database`.
//!
//! **O quê:** lê o schema pelo `--json` do binário headless e desenha; gera o SQL e mostra.
//!
//! **Onde:** o ícone do app, e a aba de Banco de dados do hub, que delega para cá.
//!
//! ## Ela mora no MESMO repo do CLI (ADR-0020), e continua falando por `--json`
//!
//! Market, optimizer e deployer têm a janela em repo próprio. Os apps da extradição nascem com
//! os dois no mesmo momento, e o repo separado custou três classes de falha nesta semana: o
//! `git clone` do release apontando para um endereço que dava 404 **em silêncio**, o pino
//! cruzado envelhecendo sozinho, e o CI da janela compilando contra uma versão do CLI diferente
//! da que o release usa.
//!
//! **Estar no mesmo repo é conveniência de distribuição, não permissão para acoplar.** A janela
//! não usa o crate `database`: ela roda o binário e lê o `--json`, como as irmãs. Há teste
//! lendo o próprio fonte para cobrar isso.

mod cli;
mod json;
mod telas;
mod terminal;

slint::include_modules!();

use slint::{ModelRc, SharedString, VecModel};

/// **O quê:** converte as tabelas do domínio nas linhas que o Slint desenha.
///
/// **Onde:** [`recarregar`].
fn linhas(tabelas: &[telas::Tabela]) -> ModelRc<TabelaUI> {
    ModelRc::new(VecModel::from(
        tabelas
            .iter()
            .map(|t| TabelaUI {
                nome: t.nome.clone().into(),
                colunas: ModelRc::new(VecModel::from(
                    t.colunas
                        .iter()
                        .map(|c| ColunaUI {
                            nome: c.nome.clone().into(),
                            tipo: c.tipo.clone().into(),
                            marcas: c.marcas.clone().into(),
                        })
                        .collect::<Vec<_>>(),
                )),
                fks: ModelRc::new(VecModel::from(
                    t.fks.iter().map(|s| SharedString::from(s.as_str())).collect::<Vec<_>>(),
                )),
                indices: ModelRc::new(VecModel::from(
                    t.indices.iter().map(|s| SharedString::from(s.as_str())).collect::<Vec<_>>(),
                )),
            })
            .collect::<Vec<_>>(),
    ))
}

/// **O quê:** os argumentos de fonte que o CLI espera, a partir do que a janela tem preenchido.
///
/// **Onde:** [`recarregar`] e a geração de SQL. Função PURA, testada.
///
/// **A ordem é a da especificidade, e é a MESMA do CLI:** `--from` ganha do banco, porque quem
/// passou um arquivo quer AQUELE schema (tipicamente um editado à mão). Divergir daqui faria a
/// janela e o terminal lerem fontes diferentes com os mesmos campos preenchidos.
fn args_da_fonte(sqlite: &str, postgres: &str, json: &str) -> Option<Vec<String>> {
    let j = json.trim();
    if !j.is_empty() {
        return Some(vec!["--from".into(), j.into()]);
    }
    let s = sqlite.trim();
    if !s.is_empty() {
        return Some(vec!["--sqlite".into(), s.into()]);
    }
    let p = postgres.trim();
    if !p.is_empty() {
        return Some(vec!["--postgres".into(), p.into()]);
    }
    None
}

/// A fonte que a janela já abre preenchida, quando alguém disse qual é.
#[derive(Debug, Default, PartialEq, Eq)]
struct FonteInicial {
    sqlite: String,
    postgres: String,
    json: String,
    /// `0` = Schema, `1` = SQL — o mesmo número da propriedade `aba` do Slint.
    aba: i32,
}

impl FonteInicial {
    fn vazia(&self) -> bool {
        self.sqlite.is_empty() && self.postgres.is_empty() && self.json.is_empty()
    }
}

/// **O quê:** lê a fonte dos argumentos, para a janela já abrir sobre o banco certo.
///
/// **Onde:** [`main`], antes de mostrar a janela — e é por aqui que **o hub delega a aba**: ele
/// já sabe qual banco a pessoa escolheu, e obrigá-la a digitar de novo na janela que ele abriu
/// seria a delegação perdendo a única informação que tinha (§37.48: o trabalho é do software).
///
/// **As flags são as MESMAS do CLI** (`--sqlite`, `--postgres`, `--from`). Um segundo vocabulário
/// só para a janela seria duas formas de dizer a mesma coisa, e a divergência apareceria no dia
/// em que o CLI ganhasse uma fonte nova.
///
/// **Flag sem valor é ignorada, não panica.** `--sqlite` no fim da linha não deve matar a janela
/// antes de ela aparecer: quem errou o comando recebe a janela vazia e um campo para preencher.
/// Função PURA — os testes passam o vetor.
fn fonte_dos_args(args: &[String]) -> FonteInicial {
    let mut f = FonteInicial::default();
    let mut i = 0;
    while i < args.len() {
        let valor = args.get(i + 1).cloned().unwrap_or_default();
        let campo = match args[i].as_str() {
            "--sqlite" => Some(&mut f.sqlite),
            "--postgres" => Some(&mut f.postgres),
            "--from" => Some(&mut f.json),
            _ => None,
        };
        // `--aba` não é fonte: é em qual tela abrir. Nome DESCONHECIDO cai no Schema, que é
        // a tela de onde tudo parte — abrir numa tela inventada seria a janela obedecendo a um
        // comando que ninguém entende.
        if args[i] == "--aba" {
            definir_aba(&mut f, &valor);
            i += if valor.is_empty() { 1 } else { 2 };
            continue;
        }
        match campo {
            // Valor que começa com `-` é a flag seguinte, não o valor desta: sem esta guarda,
            // `--sqlite --from s.json` viraria um banco chamado "--from".
            Some(c) if !valor.is_empty() && !valor.starts_with('-') => {
                *c = valor;
                i += 2;
            }
            _ => i += 1,
        }
    }
    f
}

/// **O quê:** traduz o nome da tela no número que o Slint usa.
///
/// **Onde:** [`fonte_dos_args`]. Separada para o número viver num lugar só: `1` espalhado pelo
/// código é o tipo de constante que sobrevive a uma aba nova no meio.
fn definir_aba(f: &mut FonteInicial, valor: &str) {
    f.aba = match valor {
        "sql" => 1,
        _ => 0,
    };
}

/// **O quê:** a linha de comando inteira de um subcomando: `<sub> <fonte…> <extras…>`.
///
/// **Onde:** TODOS os quatro botões. É o único lugar que monta comando, e isso é o ponto.
///
/// **Existe porque a montagem espalhada quebrou os quatro botões de uma vez.** Cada chamador
/// montava o seu vetor e a casca colava um `--json` "de brinde": os que já pediam `--json`
/// receberam a flag duas vezes e o clap recusou; o `sql`, que não aceita `--json`, recusou a
/// flag inteira. Com uma função só, quem pede JSON diz isso aqui, uma vez, e há teste.
fn argumentos(sub: &str, fonte: &[String], extras: &[&str]) -> Vec<String> {
    let mut a = vec![sub.to_string()];
    a.extend(fonte.iter().cloned());
    a.extend(extras.iter().map(|s| s.to_string()));
    a
}

/// **O quê:** relê o schema e o grafo, e escreve as propriedades da tela.
///
/// **Onde:** o botão «Ler».
///
/// **Uma leitura por documento, e o erro APARECE.** Um `default()` silencioso diria "nenhuma
/// tabela" a quem tem um banco cheio e um erro de caminho — o mesmo defeito do ADR-0015.
fn recarregar(w: &MainWindow) {
    let Some(fonte) =
        args_da_fonte(&w.get_fonte_sqlite(), &w.get_fonte_postgres(), &w.get_fonte_json())
    else {
        w.set_erro("preencha uma das três fontes acima.".into());
        return;
    };
    let args = argumentos("introspect", &fonte, &["--json"]);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    match cli::rodar(&refs).and_then(|t| telas::ler_schema(&t)) {
        Ok(tabelas) => {
            w.set_resumo(telas::resumo(&tabelas).into());
            w.set_tabelas(linhas(&tabelas));
            w.set_erro(SharedString::new());
            w.set_msg(SharedString::new());
        }
        Err(e) => {
            w.set_tabelas(ModelRc::new(VecModel::from(Vec::<TabelaUI>::new())));
            w.set_resumo(SharedString::new());
            w.set_erro(e.into());
            return;
        }
    }

    // O grafo é um SEGUNDO documento, e a falha dele não apaga o schema já lido: são duas
    // perguntas, e perder a resposta boa por causa da ruim seria trocar informação por nada.
    let args = argumentos("graph", &fonte, &["--json"]);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let arestas = cli::rodar(&refs).and_then(|t| telas::ler_grafo(&t)).unwrap_or_default();
    w.set_arestas(ModelRc::new(VecModel::from(
        arestas
            .iter()
            .map(|a| ArestaUI {
                de: a.de.clone().into(),
                para: a.para.clone().into(),
                coluna: a.coluna.clone().into(),
            })
            .collect::<Vec<_>>(),
    )));
}

/// **O quê:** gera o SQL da fonte preenchida e escreve na tela.
///
/// **Onde:** os botões «Gerar CREATE» e «Gerar migration», e a abertura direta na tela de SQL.
///
/// **É função nomeada, e não corpo de closure, porque tem DOIS chamadores.** Deixá-la dentro do
/// closure obrigaria a abertura direta a duplicar a montagem do comando — que é exatamente a
/// duplicação que já quebrou os quatro botões desta janela uma vez.
fn gerar_sql(w: &MainWindow, migration: bool) {
    let Some(fonte) =
        args_da_fonte(&w.get_fonte_sqlite(), &w.get_fonte_postgres(), &w.get_fonte_json())
    else {
        w.set_erro("preencha uma das três fontes acima.".into());
        return;
    };
    // **Sem `--json` aqui, e não por esquecimento:** o `sql` devolve SQL, e a casca roda
    // exatamente o que recebe. Foi o `--json` automático que quebrou este botão.
    let args = argumentos("sql", &fonte, if migration { &["--migration"] } else { &[] });
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match cli::rodar(&refs) {
        Ok(sql) => {
            w.set_sql(sql.into());
            w.set_erro(SharedString::new());
        }
        Err(e) => w.set_erro(e.into()),
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // **RESPONDE `--version` E SAI, antes de qualquer coisa gráfica.**
    //
    // Sem isto a janela IGNORA a flag e ABRE — e quem perguntou fica esperando. O `debugreport`
    // do CLI pergunta a versão de cada binário do ecossistema, e as três janelas irmãs tinham
    // exatamente este defeito: o `cmd_out` cortava no timeout e devolvia a primeira linha do log
    // de ambiente como se fosse o número.
    if std::env::args().skip(1).any(|a| a == "--version" || a == "-V") {
        println!("schematize-database-gui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let w = MainWindow::new()?;
    w.set_versao(format!("schematize-database-gui {}", env!("CARGO_PKG_VERSION")).into());

    // Fonte vinda da linha de comando — é assim que o hub delega a aba já sabendo o banco.
    let inicial = fonte_dos_args(&std::env::args().skip(1).collect::<Vec<_>>());
    let (tem_fonte, aba) = (!inicial.vazia(), inicial.aba);
    if tem_fonte {
        w.set_fonte_sqlite(inicial.sqlite.into());
        w.set_fonte_postgres(inicial.postgres.into());
        w.set_fonte_json(inicial.json.into());
        // E JÁ LÊ. Abrir com o campo preenchido e a tela vazia faria a pessoa clicar em «Ler»
        // para confirmar o que ela mesma acabou de informar.
        recarregar(&w);
    }
    // A tela pedida vem DEPOIS da leitura: na aba SQL, a leitura é o que habilita os botões.
    w.set_aba(aba);
    // **Abrir na tela de SQL sem o SQL seria entregar a tela pela metade.** Quem pediu esta tela
    // com uma fonte já disse o que quer; fazê-la clicar em «Gerar» para confirmar é o mesmo
    // clique vazio que o auto-read acima elimina.
    if aba == 1 && tem_fonte {
        gerar_sql(&w, false);
    }

    {
        let weak = w.as_weak();
        w.on_ler(move || {
            if let Some(w) = weak.upgrade() {
                recarregar(&w);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_gerar_sql(move |migration| {
            if let Some(w) = weak.upgrade() {
                gerar_sql(&w, migration);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_copiar_sql(move || {
            let Some(w) = weak.upgrade() else { return };
            // Copiar passa pelo TERMINAL, como toda ação desta casca: a janela não roda
            // processo direto, e há teste que reprova um `Command::new` aqui.
            let bin = cli::bin().display().to_string();
            let Some(fonte) =
                args_da_fonte(&w.get_fonte_sqlite(), &w.get_fonte_postgres(), &w.get_fonte_json())
            else {
                return;
            };
            let args = argumentos("sql", &fonte, &[]);
            let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let cmd = format!(
                "{} | (xclip -selection clipboard || wl-copy || cat)",
                cli::comando_para_terminal(&bin, &refs)
            );
            if terminal::abrir(&cmd) {
                w.set_msg("terminal aberto — o SQL foi para a área de transferência.".into());
            } else {
                w.set_msg(format!("não achei um terminal. Rode: {cmd}").into());
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_alternar_tema(move || {
            if let Some(w) = weak.upgrade() {
                w.set_dark(!w.get_dark());
            }
        });
    }

    w.run()
}

#[cfg(test)]
mod tests {
    use super::{args_da_fonte, argumentos, fonte_dos_args, FonteInicial};

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    /// **O vocabulário da janela é o do CLI.** Um segundo conjunto de nomes divergiria no dia em
    /// que o CLI ganhasse uma fonte nova, e a janela ficaria sem ela sem ninguém notar.
    #[test]
    fn a_janela_aceita_as_mesmas_flags_do_cli() {
        let f = fonte_dos_args(&v(&["--sqlite", "loja.db"]));
        assert_eq!(f.sqlite, "loja.db");
        assert_eq!(fonte_dos_args(&v(&["--postgres", "host=x"])).postgres, "host=x");
        assert_eq!(fonte_dos_args(&v(&["--from", "s.json"])).json, "s.json");
        assert_eq!(fonte_dos_args(&v(&[])), FonteInicial::default());
        assert!(fonte_dos_args(&v(&[])).vazia());
    }

    /// **Flag sem valor NÃO mata a janela antes de ela aparecer.** Um `unwrap` no argumento
    /// seguinte faria `schematize-database-gui --sqlite` panicar sem desenhar nada — e a pessoa
    /// veria um ícone que "não abre". É o §37.48: invocação não prevista é bug do software.
    #[test]
    fn flag_sem_valor_nao_derruba_a_janela() {
        assert!(fonte_dos_args(&v(&["--sqlite"])).vazia(), "flag no fim da linha");
        // O valor não engole a flag seguinte: sem a guarda, o banco se chamaria "--from".
        let f = fonte_dos_args(&v(&["--sqlite", "--from", "s.json"]));
        assert_eq!(f.sqlite, "", "`--from` não é nome de banco");
        assert_eq!(f.json, "s.json", "e a flag seguinte continua sendo lida");
    }

    /// **A tela pedida é a que abre, e nome desconhecido cai no Schema.**
    ///
    /// É por aqui que o hub delega o modal de gerar SQL: ele abre a janela já na tela certa. Sem
    /// isto, a delegação entregaria a tela de sempre e a pessoa teria de achar a aba sozinha.
    #[test]
    fn a_aba_pedida_e_a_que_abre() {
        assert_eq!(fonte_dos_args(&v(&["--aba", "sql"])).aba, 1);
        assert_eq!(fonte_dos_args(&v(&["--aba", "schema"])).aba, 0);
        assert_eq!(fonte_dos_args(&v(&[])).aba, 0, "sem pedido, a tela de partida");
        // Nome inventado NÃO abre uma tela inventada — e não panica.
        assert_eq!(fonte_dos_args(&v(&["--aba", "turbinada"])).aba, 0);
        assert_eq!(fonte_dos_args(&v(&["--aba"])).aba, 0, "flag no fim da linha");
        // E `--aba` no meio não come a fonte que vem depois.
        let f = fonte_dos_args(&v(&["--aba", "sql", "--sqlite", "loja.db"]));
        assert_eq!((f.aba, f.sqlite.as_str()), (1, "loja.db"));
        // Nem a que vem antes.
        let f = fonte_dos_args(&v(&["--sqlite", "loja.db", "--aba", "sql"]));
        assert_eq!((f.aba, f.sqlite.as_str()), (1, "loja.db"));
    }

    /// Argumento desconhecido é ignorado, e não vira fonte.
    #[test]
    fn argumento_desconhecido_nao_vira_fonte() {
        assert!(fonte_dos_args(&v(&["--turbo", "x", "loja.db"])).vazia());
    }

    /// **A ordem da fonte é a MESMA do CLI.** Divergir faria a janela e o terminal lerem
    /// fontes diferentes com os mesmos campos preenchidos — e ninguém notaria até o schema
    /// gerado sair de outro banco.
    #[test]
    fn a_ordem_da_fonte_bate_com_a_do_cli() {
        assert_eq!(args_da_fonte("a.db", "", "").unwrap(), ["--sqlite", "a.db"]);
        assert_eq!(args_da_fonte("", "conn", "").unwrap(), ["--postgres", "conn"]);
        assert_eq!(args_da_fonte("", "", "s.json").unwrap(), ["--from", "s.json"]);
        // `--from` ganha: quem passou um arquivo quer AQUELE schema.
        assert_eq!(args_da_fonte("a.db", "conn", "s.json").unwrap(), ["--from", "s.json"]);
        assert_eq!(args_da_fonte("a.db", "conn", "").unwrap(), ["--sqlite", "a.db"]);
    }

    /// Espaço em branco não é fonte — senão um campo com um espaço viraria um `--sqlite " "`.
    #[test]
    fn campo_so_com_espaco_nao_e_fonte() {
        assert!(args_da_fonte("   ", "\t", " \n ").is_none());
        assert!(args_da_fonte("", "", "").is_none());
    }

    /// **`--json` aparece UMA vez, e só onde o subcomando o aceita.**
    ///
    /// Este teste existe por um defeito visto na tela: a casca colava um `--json` no fim de
    /// tudo. Nos comandos que já o pediam, o clap recusou a flag repetida; no `sql`, que não o
    /// aceita, recusou a flag inteira. **Os quatro botões da janela quebraram de uma vez**, e
    /// nenhum teste de então falhou, porque a montagem estava espalhada por quatro lugares.
    #[test]
    fn o_json_nao_se_repete_nem_vai_onde_nao_cabe() {
        let fonte = args_da_fonte("loja.db", "", "").expect("há fonte");

        let a = argumentos("introspect", &fonte, &["--json"]);
        assert_eq!(a, ["introspect", "--sqlite", "loja.db", "--json"]);
        assert_eq!(a.iter().filter(|s| *s == "--json").count(), 1, "repetido, o clap recusa");

        assert_eq!(
            argumentos("graph", &fonte, &["--json"]).iter().filter(|s| *s == "--json").count(),
            1
        );

        // O `sql` devolve SQL. Um `--json` aqui é argumento inesperado, e o comando inteiro falha.
        let s = argumentos("sql", &fonte, &[]);
        assert_eq!(s, ["sql", "--sqlite", "loja.db"]);
        assert!(!s.contains(&"--json".to_string()), "o `sql` não aceita `--json`");
        assert_eq!(
            argumentos("sql", &fonte, &["--migration"]),
            ["sql", "--sqlite", "loja.db", "--migration"]
        );
    }

    /// A ORDEM é subcomando, fonte, extras — a que o clap exige.
    #[test]
    fn o_subcomando_vem_primeiro() {
        let a = argumentos("introspect", &["--sqlite".into(), "x".into()], &["--json"]);
        assert_eq!(a[0], "introspect", "flag antes do subcomando: o clap recusa");
        assert_eq!(a.last().unwrap(), "--json");
    }

    /// **A casca não sabe NADA do domínio** (D4 do ADR-0016).
    ///
    /// Ela mora no mesmo repo do CLI (ADR-0020), e isso é conveniência de distribuição — não
    /// permissão para acoplar. Este teste lê o próprio fonte e reprova o uso do crate `database`
    /// e qualquer `Command::new` direto.
    #[test]
    fn a_casca_nao_sabe_nada_do_dominio() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().expect("há código antes dos testes");
        assert!(
            !producao.contains("use database::") && !producao.contains("database::dominio"),
            "a janela usou o crate do domínio: ela fala com o BINÁRIO por `--json`, e depender \
             do crate a faria embutir uma versão — o bug que fez a janela irmã abrir a versão \
             antiga"
        );
        // **O varredor ignora COMENTÁRIO, e isso foi aprendido errando.**
        //
        // A primeira versão procurava `Command::new` no arquivo inteiro e reprovou — por causa
        // de um comentário MEU que dizia *"há teste que reprova um `Command::new` aqui"*.
        // Proibir a palavra proíbe explicar a regra, e é o mesmo erro que a janela do deployer
        // cometeu com `passphrase`. Comentário CITA; só a linha de código USA.
        let codigo: Vec<&str> =
            producao.lines().map(str::trim_start).filter(|l| !l.starts_with("//")).collect();
        for linha in &codigo {
            assert!(
                !linha.contains("Command::new"),
                "a janela rodou processo direto:\n  {linha}\nTudo passa pela casca \
                 (`cli`/`terminal`), que é onde o tratamento de erro vive"
            );
        }
        // Self-check: o varredor tem de VER o que procura quando ele está no código.
        assert!(
            codigo.iter().any(|l| l.contains("cli::rodar")),
            "a janela fala com o binário pela casca — se esta linha sumiu, o varredor está cego"
        );
        assert!(
            producao.contains("`Command::new`"),
            "o comentário que explica a regra tem de continuar passando — proibir a palavra \
             proibiria explicar a política"
        );
    }
}
