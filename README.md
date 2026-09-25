# schematize-database

**O desenho do banco, antes da primeira linha de aplicação.** Modela o schema relacional, lê o
banco que já existe, e gera o SQL e a migration expand-contract.

```
schematize-database introspect --sqlite loja.db          # o que já está lá
schematize-database introspect --postgres "$CONN" --json # para a janela / para um script
schematize-database sql --from schema.json               # CREATE TABLE / FK / INDEX
schematize-database sql --from schema.json --migration   # expand-contract, reversível
schematize-database graph --sqlite loja.db               # tabela = nó, FK = aresta
```

## Por que é um app, e não uma tela do hub

Nada aqui fala de skill, de overdev ou de instalação — as três coisas que definem o
`schematize`. Enquanto morou no app principal, foi o candidato mais limpo do inventário de
extradição (ADR-0018, fase **E1**): dois arquivos no CLI, dois na janela, e uma fronteira que já
era JSON.

**Ele sobe sozinho** (piso 10). A ausência dele não impede o hub de bootar — a aba de Banco de
dados vira uma tela delegada e diz o que falta.

## As duas escolhas que este repo carrega, escritas

**Postgres é lido pelo `psql` do PATH, não por um driver.** Um crate de Postgres puxa TLS e mais
quarenta dependências para um caminho que a maioria de quem usa isto nunca exercita. O custo da
escolha é um parser de saída de `psql` — e ele tem teste (`parse_psql_row`), porque parser sem
teste é a parte que quebra em silêncio.

**SQLite embutido por padrão, do sistema por opção.** `sqlite-embutido` (default) compila o
SQLite dentro do binário, que é o que os *releases* publicados usam — binário que roda em
qualquer distro sem depender da `libsqlite3` de quem baixou. `sqlite-do-sistema` linka a da
distro, e é o certo para quem constrói do fonte; o `install.sh` detecta com
`pkg-config --exists sqlite3` e escolhe sozinho.

## O `--json` é contrato

`introspect --json` e `graph --json` são lidos por código de **outro repositório** (a janela, e o
hub quando delegar a aba). Por isso o documento é escrito à mão, sem `derive`: com `derive`,
renomear um campo mudaria o contrato sem aparecer no diff, e o outro lado descobriria em produção.

As chaves **nunca** são traduzidas, e há teste de que o documento é byte a byte igual em
`en`/`pt`/`C`/`ja` — com as seis variáveis de idioma removidas, para o teste não passar por
acidente numa máquina já configurada no idioma esperado.

## Estado

Repo novo (E1 do ADR-0018). **Ainda não publicado no GitHub** — enquanto isso, ele está
declarado em `schematize_cli_rs/packaging/repos-pendentes.txt`, e o guard
`scripts/repos-de-janela.py` cobra a publicação em vez de deixar a promessa em silêncio.
