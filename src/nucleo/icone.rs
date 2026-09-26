//! Ícone do **Database** desenhado EM CÓDIGO.
//!
//! **O quê:** gera o PNG do app em qualquer tamanho, sem asset externo e sem rasterizador.
//!
//! **Onde:** [`super::desktop::instalar`], ao gravar a entrada no menu.
//!
//! ## Por que é cópia do gerador dos irmãos, com outra paleta E outra forma
//!
//! Os apps da casa precisam parecer **da mesma família** no menu (mesmo squircle, mesma
//! construção) e ao mesmo tempo ser **distinguíveis à distância**, que é como se escolhe ícone
//! numa grade. Compartilhar o desenho e trocar cor e disposição dá as duas coisas.
//!
//! - schematize: acento **azul**, grafo de 3 nós em triângulo.
//! - deployer: acento **verde-água**, 3 nós em CADEIA (origem → ponte → destino).
//! - database: acento **âmbar**, três **faixas empilhadas** — a forma de uma tabela.
//!
//! **A forma não é enfeite.** Um quarto ícone com nós redondos viraria o terceiro grafo da
//! grade, e a distinção passaria a depender só da cor — que é o que se perde primeiro em
//! 16px, em tema claro e em quem não distingue matiz.
//!
//! ## Por que EM CÓDIGO e não um `.svg` no repo
//!
//! Asset externo some, não entra no `cargo install`, e rasterizá-lo depende de ImageMagick ou
//! rsvg, que falham em SVG com frequência. Do código, o ícone sai em qualquer tamanho, nunca
//! falta, e não quebra o build. Antialiasing por supersampling — nítido de 16px a 1024px.

// Paleta: o MESMO fundo da casa, acento próprio.
const BG_TL: [f32; 3] = [0x14 as f32, 0x16 as f32, 0x1c as f32]; // #14161c (canto sup-esq)
const BG_BR: [f32; 3] = [0x1b as f32, 0x1e as f32, 0x27 as f32]; // #1b1e27 (canto inf-dir)
const ACCENT: [f32; 3] = [0xe6 as f32, 0xb2 as f32, 0x3a as f32]; // #e6b23a âmbar (Database)
const ACCENT_HI: [f32; 3] = [0xff as f32, 0xd7 as f32, 0x7a as f32]; // #ffd77a (a faixa de topo)

/// Amostras por eixo no supersampling (SS×SS por pixel) → bordas suaves sem lib de imagem.
const SS: u32 = 4;

/// Uma faixa da "tabela": centro em Y, meia-altura, e a cor. Em FRAÇÃO do lado.
///
/// **Onde:** [`rgba`] e [`sample`].
///
/// **A de cima é mais clara e mais fina** porque é o CABEÇALHO — é o que faz a pilha ler como
/// tabela e não como três traços iguais. Três iguais seriam um menu hambúrguer, que é o ícone
/// de "abrir menu" em metade dos aplicativos do mundo.
const FAIXAS: [(f32, f32, [f32; 3]); 3] = [
    (0.325, 0.055, ACCENT_HI), // cabeçalho
    (0.500, 0.070, ACCENT),    // linha
    (0.675, 0.070, ACCENT),    // linha
];

/// Meia-largura das faixas, em fração do lado.
const FAIXA_W: f32 = 0.270;
/// Raio das pontas arredondadas das faixas, em fração do lado.
const FAIXA_R: f32 = 0.030;

/// Onde passa a DIVISÓRIA DE COLUNA, em fração do lado (X absoluto), e sua meia-largura.
///
/// **Sem ela o ícone é um menu hambúrguer, e essa é a razão de ela existir.** Três faixas
/// empilhadas é o botão de "abrir menu" em metade dos aplicativos do mundo; a diferença entre
/// uma tabela e um menu é justamente a COLUNA. O corte atravessa as duas linhas de dados e
/// **não** o cabeçalho — que é como uma tabela de verdade se desenha, e o que mantém a pilha
/// legível quando o ícone encolhe para 16px e a divisória some no antialiasing.
const COLUNA_X: f32 = 0.605;
const COLUNA_W: f32 = 0.022;

/// **O quê:** o ícone em RGBA no tamanho `n` (px). Devolve `(bytes, w, h)`. PURO e determinístico.
///
/// **Onde:** [`write_png`], e os testes — que podem afirmar pixels sem tocar no disco.
pub fn rgba(n: u32) -> (Vec<u8>, u32, u32) {
    let nf = n as f32;
    let radius = nf * 0.227; // rx=232/1024, o mesmo squircle da casa
    let mut buf = vec![0u8; (n * n * 4) as usize];
    let inv_ss2 = 1.0 / (SS * SS) as f32;

    for y in 0..n {
        for x in 0..n {
            let (mut r, mut g, mut b, mut a) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let px = x as f32 + (sx as f32 + 0.5) / SS as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / SS as f32;
                    if let Some(c) = sample(px, py, nf, radius) {
                        r += c[0];
                        g += c[1];
                        b += c[2];
                        a += 255.0;
                    }
                }
            }
            let idx = ((y * n + x) * 4) as usize;
            let cover = a * inv_ss2; // 0..255 — o alpha final é a cobertura
            if cover > 0.0 {
                // Média só sobre os subpixels COBERTOS: dividir pelo total criaria um halo
                // escuro na borda, porque os vazios entrariam como preto.
                let covered = a / 255.0;
                buf[idx] = (r / covered).round().clamp(0.0, 255.0) as u8;
                buf[idx + 1] = (g / covered).round().clamp(0.0, 255.0) as u8;
                buf[idx + 2] = (b / covered).round().clamp(0.0, 255.0) as u8;
                buf[idx + 3] = cover.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    (buf, n, n)
}

/// **O quê:** a cor OPACA de um subpixel, ou `None` fora do squircle (transparente).
///
/// **Onde:** [`rgba`]. Ordem de pintura: fundo (gradiente) → faixas.
fn sample(px: f32, py: f32, nf: f32, radius: f32) -> Option<[f32; 3]> {
    if !inside_rounded(px, py, nf, radius) {
        return None;
    }
    let t = ((px + py) / (2.0 * nf)).clamp(0.0, 1.0);
    let mut color = [
        BG_TL[0] + (BG_BR[0] - BG_TL[0]) * t,
        BG_TL[1] + (BG_BR[1] - BG_TL[1]) * t,
        BG_TL[2] + (BG_BR[2] - BG_TL[2]) * t,
    ];
    for (i, (cy, meia_h, c)) in FAIXAS.iter().enumerate() {
        if dentro_da_faixa(px, py, nf, *cy, *meia_h) {
            // A divisória é o FUNDO aparecendo através da faixa — e só nas linhas de dados.
            // No cabeçalho ela não passa, como numa tabela de verdade.
            let na_coluna = i > 0 && (px / nf - COLUNA_X).abs() <= COLUNA_W;
            if !na_coluna {
                color = *c;
            }
            break;
        }
    }
    Some(color)
}

/// **O quê:** o ponto cai numa faixa de cantos arredondados, centrada em `cy`?
///
/// **Onde:** [`sample`]. Separada porque é a geometria da forma — e é o que um teste afirma
/// sem ter de ler pixel.
fn dentro_da_faixa(px: f32, py: f32, nf: f32, cy: f32, meia_h: f32) -> bool {
    let (x, y) = (px / nf - 0.5, py / nf - cy);
    let (hw, hh, r) = (FAIXA_W, meia_h, FAIXA_R.min(meia_h));
    // Retângulo arredondado: encolhe pelo raio, mede a distância ao retângulo interno.
    let dx = (x.abs() - (hw - r)).max(0.0);
    let dy = (y.abs() - (hh - r)).max(0.0);
    x.abs() <= hw && y.abs() <= hh && dx * dx + dy * dy <= r * r
}

/// **O quê:** ponto dentro de um quadrado `0..n` com cantos de raio `r`.
///
/// **Onde:** [`sample`], como recorte do squircle.
fn inside_rounded(px: f32, py: f32, n: f32, r: f32) -> bool {
    if px < 0.0 || py < 0.0 || px > n || py > n {
        return false;
    }
    let cx = px.clamp(r, n - r);
    let cy = py.clamp(r, n - r);
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= r * r
}

/// **O quê:** escreve o ícone (tamanho `n`) como PNG em `path`, criando os diretórios-pai.
///
/// **Onde:** [`write_hicolor`] e [`install_all`].
pub fn write_png(path: &std::path::Path, n: u32) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let (data, w, h) = rgba(n);
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(std::io::Error::other)?;
    writer.write_image_data(&data).map_err(std::io::Error::other)?;
    Ok(())
}

/// Nome do arquivo de ícone. Um lugar só: se divergir do `Icon=` do `.desktop`, o menu mostra
/// um quadrado cinza e ninguém liga a causa ao nome.
pub const NOME: &str = "schematize-database";

/// Tamanhos hicolor padrão (freedesktop).
pub const HICOLOR_SIZES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];

/// **O quê:** gera a árvore hicolor completa em `base`. Devolve os caminhos escritos.
///
/// **Onde:** [`install_all`].
pub fn write_hicolor(base: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    for n in HICOLOR_SIZES {
        let p = base.join(format!("{n}x{n}")).join("apps").join(format!("{NOME}.png"));
        write_png(&p, n)?;
        out.push(p);
    }
    Ok(out)
}

/// **O quê:** instala o ícone em todos os locais que os ambientes consultam, e devolve o
/// caminho ABSOLUTO do 256px — o que vai no `Icon=`.
///
/// **Onde:** [`super::desktop::instalar_com_gui`].
///
/// **Absoluto no `Icon=` de propósito:** no Wayland o dock casa a janela ao `.desktop`, e um
/// nome de tema depende de cache de ícones que pode estar velho ou quebrado. Os extras são
/// best-effort — o hicolor é o que importa.
pub fn install_all(home: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let icons = home.join(".local/share/icons/hicolor");
    write_hicolor(&icons)?;
    let p256 = icons.join("256x256").join("apps").join(format!("{NOME}.png"));
    for extra in [
        home.join(format!(".local/share/pixmaps/{NOME}.png")),
        home.join(format!(".icons/{NOME}.png")),
        home.join(format!(".local/share/icons/{NOME}.png")),
    ] {
        let _ = write_png(&extra, 256);
    }
    Ok(p256)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O canto é TRANSPARENTE e o centro é OPACO — é o squircle existindo.
    #[test]
    fn o_squircle_recorta_os_cantos() {
        let (buf, n, _) = rgba(64);
        let alpha = |x: u32, y: u32| buf[((y * n + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "o canto tem de ser transparente");
        assert_eq!(alpha(63, 63), 0);
        assert_eq!(alpha(32, 32), 255, "o centro tem de ser opaco");
    }

    /// **As três faixas existem, e a de cima é a clara.**
    ///
    /// Sem esta asserção, um erro de geometria produziria um quadrado âmbar liso — que abre,
    /// instala e parece um ícone, e só não é o ícone deste app.
    #[test]
    fn a_pilha_tem_tres_faixas_e_cabecalho_claro() {
        let n = 256u32;
        let (buf, _, _) = rgba(n);
        let px = |x: u32, y: u32| {
            let i = ((y * n + x) * 4) as usize;
            [buf[i], buf[i + 1], buf[i + 2]]
        };
        let meio = n / 2;
        let em = |frac: f32| px(meio, (frac * n as f32) as u32);
        let cab = em(FAIXAS[0].0);
        let l1 = em(FAIXAS[1].0);
        let l2 = em(FAIXAS[2].0);
        assert_eq!(l1, l2, "as duas linhas são da mesma cor");
        assert_ne!(cab, l1, "o cabeçalho tem de se distinguir da linha");
        assert!(cab[0] > l1[0] && cab[1] > l1[1], "o cabeçalho é o MAIS CLARO: {cab:?} vs {l1:?}");
        // E entre as faixas há FUNDO — senão seria um bloco só.
        let vao = em((FAIXAS[0].0 + FAIXAS[1].0) / 2.0);
        assert_ne!(vao, l1, "sem vão entre as faixas, a pilha vira um retângulo");
    }

    /// **A DIVISÓRIA DE COLUNA é o que separa uma tabela de um menu hambúrguer.**
    ///
    /// Três faixas empilhadas é o botão de "abrir menu" em metade dos aplicativos do mundo.
    /// Sem este corte, o ícone deste app seria aquele botão — e nenhum outro teste notaria,
    /// porque a pilha continuaria com três faixas, cabeçalho claro e vão entre elas.
    #[test]
    fn a_divisoria_de_coluna_existe_e_poupa_o_cabecalho() {
        let n = 256u32;
        let (buf, _, _) = rgba(n);
        let px = |x: u32, y: u32| {
            let i = ((y * n + x) * 4) as usize;
            [buf[i], buf[i + 1], buf[i + 2]]
        };
        let col_x = (COLUNA_X * n as f32) as u32;
        let linha_y = (FAIXAS[1].0 * n as f32) as u32;
        let dentro_x = (0.5 * n as f32) as u32;

        assert_ne!(
            px(col_x, linha_y),
            px(dentro_x, linha_y),
            "a divisória não apareceu na linha de dados — o ícone é um menu hambúrguer"
        );
        // No CABEÇALHO ela não passa: é o que faz a pilha ler como tabela, e o que a mantém
        // legível em 16px, onde a divisória some no antialiasing.
        let cab_y = (FAIXAS[0].0 * n as f32) as u32;
        assert_eq!(
            px(col_x, cab_y),
            px(dentro_x, cab_y),
            "a divisória atravessou o cabeçalho — numa tabela ela não atravessa"
        );
    }

    /// **A forma é a de uma TABELA, não a de um grafo.** É o que distingue este ícone dos dois
    /// irmãos numa grade de 16px, onde a cor é a primeira coisa que se perde.
    #[test]
    fn a_faixa_e_larga_e_achatada() {
        for (_, meia_h, _) in FAIXAS {
            assert!(FAIXA_W > meia_h * 2.0, "faixa mais alta que larga não lê como linha");
        }
        // No centro vertical de uma faixa, a borda esquerda está DENTRO do squircle: uma faixa
        // que vaza o recorte apareceria cortada em quadrado.
        let nf = 256.0;
        let x = (0.5 - FAIXA_W) * nf;
        assert!(inside_rounded(x, FAIXAS[1].0 * nf, nf, nf * 0.227), "a faixa vaza o squircle");
    }

    /// Todo tamanho hicolor sai, e sai com o número de bytes certo.
    #[test]
    fn sai_em_todo_tamanho_do_hicolor() {
        for n in HICOLOR_SIZES {
            let (buf, w, h) = rgba(n);
            assert_eq!((w, h), (n, n));
            assert_eq!(buf.len(), (n * n * 4) as usize, "tamanho {n}");
        }
    }
}
