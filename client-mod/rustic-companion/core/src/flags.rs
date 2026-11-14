//! The language screen's flags, drawn in code (no third-party artwork): each flag is a few
//! layers (bands, crosses, discs, triangles, a canton holding another flag), simplified for a
//! picture a few dozen pixels wide (no stars or coats of arms). A language is shown with its
//! region's flag (`fr_ca` with Canada's); the game's regional and constructed languages have
//! their own where a well-known one exists (Esperanto's, Ido's), a historic variant its country's,
//! and the rest a symbol tile of their own (Toki Pona's sun, Valyrian's): every one of the game's
//! languages has a picture ([`flag_for`] is None only for a code the game does not have).
use crate::title::Rgba;

/// One layer, in the flag's unit square (x right, y down, 0-1).
#[derive(Clone, Copy, Debug)]
pub enum Layer {
    Fill(u32),
    /// Horizontal bands top to bottom, by weight.
    H(&'static [(u32, f32)]),
    /// Vertical bands left to right, by weight.
    V(&'static [(u32, f32)]),
    /// A rectangle `[x, y, w, h]`.
    Rect([f32; 4], u32),
    /// A disc at `[cx, cy]` of radius `r` (a fraction of the height).
    Disc([f32; 2], f32, u32),
    /// A centred cross whose arms are `t` thick (a fraction of the height) and reach the edges.
    Cross(f32, u32),
    /// A short cross (Switzerland's): arms `t` thick and `len` long, both of the height.
    Plus(f32, f32, u32),
    /// A Nordic cross centred at `x` (fraction of the width), `t` thick.
    Nordic(f32, f32, u32),
    /// Both diagonals, `t` thick.
    Saltire(f32, u32),
    /// The diagonal from the top left to the bottom right, `t` thick.
    Diagonal(f32, u32),
    /// A triangle from the hoist, its tip at `depth` of the width.
    Triangle(f32, u32),
    /// A diamond inset by `inset` of the height.
    Diamond(f32, u32),
    /// Another flag drawn in `[x, y, w, h]`.
    Sub([f32; 4], &'static [Layer]),
    /// A polygon through these points (fractions of the width and the height).
    Poly(&'static [[f32; 2]], u32),
}

const RED: u32 = 0xD52B1E;
const WHITE: u32 = 0xFFFFFF;
const BLUE: u32 = 0x0039A6;
const NAVY: u32 = 0x012169;
const GREEN: u32 = 0x009246;
const YELLOW: u32 = 0xFFCD00;
const BLACK: u32 = 0x111111;
const SKY: u32 = 0x74ACDF;
const ORANGE: u32 = 0xFF9933;

const UNION: &[Layer] = &[
    Layer::Fill(NAVY),
    Layer::Saltire(0.2, WHITE),
    Layer::Saltire(0.07, 0xC8102E),
    Layer::Cross(0.33, WHITE),
    Layer::Cross(0.2, 0xC8102E),
];
const ENGLAND: &[Layer] = &[Layer::Fill(WHITE), Layer::Cross(0.2, 0xC8102E)];
const US: &[Layer] = &[
    Layer::H(&[
        (0xB22234, 1.0),
        (WHITE, 1.0),
        (0xB22234, 1.0),
        (WHITE, 1.0),
        (0xB22234, 1.0),
        (WHITE, 1.0),
        (0xB22234, 1.0),
        (WHITE, 1.0),
        (0xB22234, 1.0),
        (WHITE, 1.0),
        (0xB22234, 1.0),
        (WHITE, 1.0),
        (0xB22234, 1.0),
    ]),
    Layer::Rect([0.0, 0.0, 0.42, 0.54], 0x3C3B6E),
];
const FRANCE: &[Layer] = &[Layer::V(&[(0x0055A4, 1.0), (WHITE, 1.0), (0xEF4135, 1.0)])];
const GERMANY: &[Layer] = &[Layer::H(&[(BLACK, 1.0), (0xDD0000, 1.0), (YELLOW, 1.0)])];
const SPAIN: &[Layer] = &[Layer::H(&[
    (0xAA151B, 1.0),
    (0xF1BF00, 2.0),
    (0xAA151B, 1.0),
])];
const ITALY: &[Layer] = &[Layer::V(&[(GREEN, 1.0), (WHITE, 1.0), (0xCE2B37, 1.0)])];
const MEXICO: &[Layer] = &[
    Layer::V(&[(0x006847, 1.0), (WHITE, 1.0), (0xCE1126, 1.0)]),
    Layer::Disc([0.5, 0.5], 0.13, 0x8C6A32),
];
const RUSSIA: &[Layer] = &[Layer::H(&[(WHITE, 1.0), (BLUE, 1.0), (RED, 1.0)])];
const NETHERLANDS: &[Layer] = &[Layer::H(&[(0xAE1C28, 1.0), (WHITE, 1.0), (0x21468B, 1.0)])];
const BELGIUM: &[Layer] = &[Layer::V(&[(BLACK, 1.0), (0xFDDA24, 1.0), (0xEF3340, 1.0)])];
const SWEDEN: &[Layer] = &[Layer::Fill(0x006AA7), Layer::Nordic(0.36, 0.2, 0xFECC00)];
const NORWAY: &[Layer] = &[
    Layer::Fill(0xBA0C2F),
    Layer::Nordic(0.36, 0.25, WHITE),
    Layer::Nordic(0.36, 0.12, 0x00205B),
];
const DENMARK: &[Layer] = &[Layer::Fill(0xC8102E), Layer::Nordic(0.36, 0.14, WHITE)];
const FINLAND: &[Layer] = &[Layer::Fill(WHITE), Layer::Nordic(0.36, 0.22, 0x002F6C)];
const CHINA: &[Layer] = &[
    Layer::Fill(0xEE1C25),
    Layer::Disc([0.17, 0.27], 0.15, 0xFFFF00),
];
const UKRAINE: &[Layer] = &[Layer::H(&[(0x0057B7, 1.0), (0xFFD700, 1.0)])];
const BRAZIL: &[Layer] = &[
    Layer::Fill(0x009C3B),
    Layer::Diamond(0.1, 0xFFDF00),
    Layer::Disc([0.5, 0.5], 0.25, 0x002776),
];
const PORTUGAL: &[Layer] = &[
    Layer::V(&[(0x006600, 2.0), (0xFF0000, 3.0)]),
    Layer::Disc([0.4, 0.5], 0.2, 0xFFCC00),
];
const POLAND: &[Layer] = &[Layer::H(&[(WHITE, 1.0), (0xDC143C, 1.0)])];
const CANADA: &[Layer] = &[
    Layer::V(&[(0xD80621, 1.0), (WHITE, 2.0), (0xD80621, 1.0)]),
    Layer::Diamond(0.28, 0xD80621),
];
const AUSTRIA: &[Layer] = &[Layer::H(&[(0xED2939, 1.0), (WHITE, 1.0), (0xED2939, 1.0)])];
const SWITZERLAND: &[Layer] = &[Layer::Fill(0xDA291C), Layer::Plus(0.2, 0.62, WHITE)];
const SERBIA: &[Layer] = &[Layer::H(&[(0xC6363C, 1.0), (0x0C4076, 1.0), (WHITE, 1.0)])];
const MALAYSIA: &[Layer] = &[
    Layer::H(&[
        (0xCC0001, 1.0),
        (WHITE, 1.0),
        (0xCC0001, 1.0),
        (WHITE, 1.0),
        (0xCC0001, 1.0),
        (WHITE, 1.0),
        (0xCC0001, 1.0),
        (WHITE, 1.0),
        (0xCC0001, 1.0),
        (WHITE, 1.0),
        (0xCC0001, 1.0),
        (WHITE, 1.0),
        (0xCC0001, 1.0),
        (WHITE, 1.0),
    ]),
    Layer::Rect([0.0, 0.0, 0.5, 0.57], 0x010066),
    Layer::Disc([0.22, 0.28], 0.17, 0xFFCC00),
    Layer::Disc([0.27, 0.28], 0.15, 0x010066),
];
const INDIA: &[Layer] = &[
    Layer::H(&[(ORANGE, 1.0), (WHITE, 1.0), (0x138808, 1.0)]),
    Layer::Disc([0.5, 0.5], 0.12, 0x000080),
    Layer::Disc([0.5, 0.5], 0.09, WHITE),
];
const NIGERIA: &[Layer] = &[Layer::V(&[(0x008751, 1.0), (WHITE, 1.0), (0x008751, 1.0)])];
const MOROCCAN_GREEN: u32 = 0x006233;
const PHILIPPINES: &[Layer] = &[
    Layer::H(&[(0x0038A8, 1.0), (0xCE1126, 1.0)]),
    Layer::Triangle(0.43, WHITE),
    Layer::Disc([0.15, 0.5], 0.1, 0xFCD116),
];
const ESPERANTO: &[Layer] = &[
    Layer::Fill(MOROCCAN_GREEN),
    Layer::Rect([0.0, 0.0, 0.33, 0.5], WHITE),
    Layer::Disc([0.165, 0.25], 0.12, MOROCCAN_GREEN),
];
const NEUTRAL: u32 = 0x3A3B42;

/// The flag of region `region` (two letters, upper case) or of a regional or constructed
/// language's own code (`ca_es`, `eo_uy`, `en_pt`...).
fn design(code: &str) -> Option<&'static [Layer]> {
    // Languages with a flag of their own.
    let own: Option<&'static [Layer]> = match code {
        "eo_uy" => Some(ESPERANTO),
        "en_pt" => Some(&[
            Layer::Fill(BLACK),
            Layer::Saltire(0.08, 0xE8E8E8),
            Layer::Disc([0.5, 0.42], 0.24, 0xE8E8E8),
            Layer::Disc([0.43, 0.4], 0.05, BLACK),
            Layer::Disc([0.57, 0.4], 0.05, BLACK),
        ]),
        "enp" | "enws" => Some(ENGLAND),
        "lol_us" => Some(US),
        "cy_gb" => Some(&[
            Layer::H(&[(WHITE, 1.0), (0x00AB39, 1.0)]),
            Layer::Disc([0.5, 0.5], 0.27, 0xD30731),
        ]),
        "gd_gb" => Some(&[Layer::Fill(0x0065BD), Layer::Saltire(0.18, WHITE)]),
        "kw_gb" => Some(&[Layer::Fill(BLACK), Layer::Cross(0.2, WHITE)]),
        "br_fr" => Some(&[
            Layer::H(&[
                (BLACK, 1.0),
                (WHITE, 1.0),
                (BLACK, 1.0),
                (WHITE, 1.0),
                (BLACK, 1.0),
                (WHITE, 1.0),
                (BLACK, 1.0),
                (WHITE, 1.0),
                (BLACK, 1.0),
            ]),
            Layer::Rect([0.0, 0.0, 0.42, 0.56], WHITE),
        ]),
        "oc_fr" => Some(&[
            Layer::Fill(0xD10000),
            Layer::Disc([0.5, 0.5], 0.28, 0xFFD700),
        ]),
        "eu_es" => Some(&[
            Layer::Fill(0xD52B1E),
            Layer::Saltire(0.16, 0x009B48),
            Layer::Cross(0.12, WHITE),
        ]),
        "ca_es" | "val_es" => Some(if code == "val_es" {
            &[
                Layer::H(&[
                    (0xFCDD09, 1.0),
                    (0xDA121A, 1.0),
                    (0xFCDD09, 1.0),
                    (0xDA121A, 1.0),
                    (0xFCDD09, 1.0),
                    (0xDA121A, 1.0),
                    (0xFCDD09, 1.0),
                    (0xDA121A, 1.0),
                    (0xFCDD09, 1.0),
                ]),
                Layer::Rect([0.0, 0.0, 0.22, 1.0], 0x0039A6),
            ]
        } else {
            &[Layer::H(&[
                (0xFCDD09, 1.0),
                (0xDA121A, 1.0),
                (0xFCDD09, 1.0),
                (0xDA121A, 1.0),
                (0xFCDD09, 1.0),
                (0xDA121A, 1.0),
                (0xFCDD09, 1.0),
                (0xDA121A, 1.0),
                (0xFCDD09, 1.0),
            ])]
        }),
        "gl_es" => Some(&[Layer::Fill(WHITE), Layer::Diagonal(0.24, 0x0099CC)]),
        "ast_es" => Some(&[Layer::Fill(0x0066FF), Layer::Plus(0.1, 0.6, 0xFFCC00)]),
        "esan" => Some(&[Layer::H(&[(0x007A33, 1.0), (WHITE, 1.0), (0x007A33, 1.0)])]),
        "qcb_es" => Some(&[Layer::H(&[
            (0xFCD116, 2.0),
            (0x003893, 1.0),
            (0xCE1126, 1.0),
        ])]),
        "fy_nl" => Some(&[
            Layer::Fill(WHITE),
            Layer::Diagonal(0.12, 0x0066CC),
            Layer::Disc([0.25, 0.65], 0.09, 0xDD0000),
            Layer::Disc([0.7, 0.3], 0.09, 0xDD0000),
        ]),
        "se_no" => Some(&[
            Layer::V(&[
                (0xD52B1E, 6.0),
                (0x007229, 1.0),
                (0xFFCE00, 1.0),
                (0x0035AD, 6.0),
            ]),
            Layer::Disc([0.43, 0.5], 0.3, 0xD52B1E),
            Layer::Disc([0.43, 0.5], 0.22, 0x0035AD),
        ]),
        "haw_us" => Some(&[
            Layer::H(&[
                (WHITE, 1.0),
                (0xC8102E, 1.0),
                (0x012169, 1.0),
                (WHITE, 1.0),
                (0xC8102E, 1.0),
                (0x012169, 1.0),
                (WHITE, 1.0),
                (0xC8102E, 1.0),
            ]),
            Layer::Sub([0.0, 0.0, 0.5, 0.5], UNION),
        ]),
        "tt_ru" => Some(&[Layer::H(&[(0x009C3B, 7.0), (WHITE, 1.0), (0xD52B1E, 7.0)])]),
        "ba_ru" => Some(&[
            Layer::H(&[(0x0066B3, 1.0), (WHITE, 1.0), (0x00AF66, 1.0)]),
            Layer::Disc([0.5, 0.5], 0.16, 0xFFD100),
        ]),
        "sah_sah" => Some(&[
            Layer::H(&[
                (0x4FA4E3, 8.0),
                (WHITE, 1.0),
                (0xD52B1E, 1.0),
                (0x009C3B, 1.0),
            ]),
            Layer::Disc([0.5, 0.38], 0.2, WHITE),
        ]),
        "cv_cu" => Some(&[
            Layer::H(&[(0xFFD700, 3.0), (0xA22B38, 1.0)]),
            Layer::Diamond(0.3, 0xA22B38),
        ]),
        "fur_it" => Some(&[
            Layer::Fill(0x0066CC),
            Layer::Disc([0.5, 0.5], 0.25, 0xFFCC00),
        ]),
        "vec_it" => Some(&[
            Layer::Fill(0xAA0000),
            Layer::Disc([0.5, 0.5], 0.25, 0xFFCC00),
        ]),
        "lmo" => Some(&[Layer::Fill(WHITE), Layer::Disc([0.5, 0.5], 0.26, 0x009246)]),
        "ksh" => Some(&[Layer::H(&[(0xE30613, 1.0), (WHITE, 1.0)])]),
        "bar" => Some(&[Layer::H(&[
            (WHITE, 1.0),
            (0x0098D4, 1.0),
            (WHITE, 1.0),
            (0x0098D4, 1.0),
        ])]),
        "sxu" => Some(&[Layer::H(&[(WHITE, 1.0), (0x009B3A, 1.0)])]),
        "fra_de" => Some(&[Layer::H(&[(0xD52B1E, 1.0), (WHITE, 1.0)])]),
        "szl" => Some(&[Layer::H(&[(0xFFCC00, 1.0), (0x0033A0, 1.0)])]),
        "isv" => Some(&[Layer::H(&[(0x0C4076, 1.0), (WHITE, 1.0), (0xC6363C, 1.0)])]),
        "jbo_en" => Some(&[
            Layer::Fill(WHITE),
            Layer::Disc([0.5, 0.5], 0.4, 0x0055A4),
            Layer::Disc([0.5, 0.5], 0.3, WHITE),
            Layer::Disc([0.5, 0.5], 0.14, 0x0055A4),
        ]),
        "tlh_aa" => Some(&[
            Layer::Fill(BLACK),
            Layer::Triangle(0.62, 0x9B0F0F),
            Layer::Disc([0.25, 0.5], 0.12, BLACK),
        ]),
        "qya_aa" => Some(&[
            Layer::Fill(0x0F3B2E),
            Layer::Disc([0.5, 0.5], 0.22, 0xE6E2C8),
            Layer::Disc([0.5, 0.5], 0.14, 0x0F3B2E),
        ]),
        "en_ud" => Some(UNION),
        "la_la" => Some(&[
            Layer::Fill(0x7A1712),
            Layer::Disc([0.5, 0.5], 0.24, 0xE0B341),
        ]),
        "rpr" => Some(&[Layer::H(&[(BLACK, 1.0), (0xFFD700, 1.0), (WHITE, 1.0)])]),
        // Ido's flag (Wikimedia Commons, Flag_of_Ido.svg, public domain): a white star on light
        // blue, simplified to its outline.
        "io_en" => Some(&[
            Layer::Fill(0x3399FF),
            Layer::Poly(&[[0.5, 0.06], [0.93, 0.92], [0.07, 0.92]], WHITE),
            Layer::Poly(&[[0.5, 0.94], [0.07, 0.22], [0.93, 0.22]], WHITE),
        ]),
        // Toki Pona has no flag: its sign (a light yellow sun under three rays, outlined in navy:
        // Wikimedia Commons, Toki_pona.svg, public domain) on navy.
        "tok" => Some(&[
            Layer::Fill(0x000099),
            Layer::Disc([0.5, 0.6], 0.27, 0xFFFF63),
            Layer::Rect([0.48, 0.08, 0.04, 0.18], 0xFFFF63),
            Layer::Poly(
                &[[0.30, 0.16], [0.335, 0.13], [0.40, 0.29], [0.365, 0.32]],
                0xFFFF63,
            ),
            Layer::Poly(
                &[[0.70, 0.16], [0.665, 0.13], [0.60, 0.29], [0.635, 0.32]],
                0xFFFF63,
            ),
        ]),
        // Valyrian, a language of fiction with no flag: a symbol tile (a red diamond on black).
        "vp_vl" => Some(&[
            Layer::Fill(0x17171B),
            Layer::Diamond(0.24, 0xA3201B),
            Layer::Diamond(0.36, 0x17171B),
        ]),
        _ => None,
    };
    if own.is_some() {
        return own;
    }
    let region = code.split('_').nth(1).unwrap_or("").to_ascii_uppercase();
    let region = match code {
        "zlm_arab" => "MY".to_owned(),
        "be_latn" => "BY".to_owned(),
        "sr_sp" | "sr_cs" => "RS".to_owned(),
        "lzh" => "CN".to_owned(),
        "nah" | "pls" => "MX".to_owned(),
        "vro" => "EE".to_owned(),
        "brb" => "BE".to_owned(),
        "ovd" => "SE".to_owned(),
        // Indonesian in its old spelling: Indonesia's.
        "qid" => "ID".to_owned(),
        "got_de" | "yi_de" | "nds_de" => "DE".to_owned(),
        "hn_no" => "NO".to_owned(),
        "hal_ua" | "ry_ua" => "UA".to_owned(),
        "li_li" => "NL".to_owned(),
        _ => region,
    };
    Some(match region.as_str() {
        "US" => US,
        "GB" | "UK" => UNION,
        "AU" => &[
            Layer::Fill(NAVY),
            Layer::Sub([0.0, 0.0, 0.5, 0.5], UNION),
            Layer::Disc([0.25, 0.76], 0.1, WHITE),
            Layer::Disc([0.75, 0.3], 0.05, WHITE),
            Layer::Disc([0.82, 0.6], 0.05, WHITE),
            Layer::Disc([0.68, 0.8], 0.05, WHITE),
        ],
        "NZ" => &[
            Layer::Fill(NAVY),
            Layer::Sub([0.0, 0.0, 0.5, 0.5], UNION),
            Layer::Disc([0.75, 0.3], 0.06, 0xC8102E),
            Layer::Disc([0.82, 0.55], 0.06, 0xC8102E),
            Layer::Disc([0.74, 0.8], 0.06, 0xC8102E),
        ],
        "CA" => CANADA,
        "FR" => FRANCE,
        "DE" => GERMANY,
        "AT" => AUSTRIA,
        "CH" => SWITZERLAND,
        "ES" => SPAIN,
        "IT" => ITALY,
        "PT" => PORTUGAL,
        "BR" => BRAZIL,
        "MX" | "TZO" => MEXICO,
        "AR" => &[
            Layer::H(&[(SKY, 1.0), (WHITE, 1.0), (SKY, 1.0)]),
            Layer::Disc([0.5, 0.5], 0.1, 0xF6B40E),
        ],
        "CL" => &[
            Layer::H(&[(WHITE, 1.0), (0xD52B1E, 1.0)]),
            Layer::Rect([0.0, 0.0, 0.33, 0.5], 0x0039A6),
            Layer::Disc([0.165, 0.25], 0.09, WHITE),
        ],
        "EC" => &[
            Layer::H(&[(0xFFDD00, 2.0), (0x034EA2, 1.0), (0xED1C24, 1.0)]),
            Layer::Disc([0.5, 0.5], 0.13, 0x6B4423),
        ],
        "UY" => &[
            Layer::H(&[
                (WHITE, 1.0),
                (0x0038A8, 1.0),
                (WHITE, 1.0),
                (0x0038A8, 1.0),
                (WHITE, 1.0),
                (0x0038A8, 1.0),
                (WHITE, 1.0),
                (0x0038A8, 1.0),
                (WHITE, 1.0),
            ]),
            Layer::Rect([0.0, 0.0, 0.38, 0.56], WHITE),
            Layer::Disc([0.19, 0.28], 0.15, 0xFCD116),
        ],
        "VE" => &[Layer::H(&[
            (0xFFCC00, 1.0),
            (0x00247D, 1.0),
            (0xCF142B, 1.0),
        ])],
        "NL" => NETHERLANDS,
        "BE" => BELGIUM,
        "LU" => &[Layer::H(&[(0xEF3340, 1.0), (WHITE, 1.0), (0x00A3E0, 1.0)])],
        "DK" => DENMARK,
        "NO" => NORWAY,
        "SE" => SWEDEN,
        "FI" => FINLAND,
        "IS" => &[
            Layer::Fill(0x02529C),
            Layer::Nordic(0.36, 0.25, WHITE),
            Layer::Nordic(0.36, 0.12, 0xDC1E35),
        ],
        "FO" => &[
            Layer::Fill(WHITE),
            Layer::Nordic(0.36, 0.25, 0x0065BD),
            Layer::Nordic(0.36, 0.12, 0xEF303E),
        ],
        "EE" => &[Layer::H(&[(0x0072CE, 1.0), (BLACK, 1.0), (WHITE, 1.0)])],
        "LV" => &[Layer::H(&[(0x9E3039, 2.0), (WHITE, 1.0), (0x9E3039, 2.0)])],
        "LT" => &[Layer::H(&[
            (0xFDB913, 1.0),
            (0x006A44, 1.0),
            (0xC1272D, 1.0),
        ])],
        "PL" => POLAND,
        "CZ" => &[
            Layer::H(&[(WHITE, 1.0), (0xD7141A, 1.0)]),
            Layer::Triangle(0.5, 0x11457E),
        ],
        "SK" => &[
            Layer::H(&[(WHITE, 1.0), (0x0B4EA2, 1.0), (0xEE1C25, 1.0)]),
            Layer::Disc([0.33, 0.5], 0.2, 0xEE1C25),
        ],
        "SI" => &[
            Layer::H(&[(WHITE, 1.0), (0x0000FF, 1.0), (0xFF0000, 1.0)]),
            Layer::Disc([0.3, 0.38], 0.14, 0x0000FF),
        ],
        "HR" => &[
            Layer::H(&[(0xFF0000, 1.0), (WHITE, 1.0), (0x171796, 1.0)]),
            Layer::Rect([0.42, 0.25, 0.16, 0.38], 0xFF0000),
            Layer::Rect([0.46, 0.3, 0.08, 0.28], WHITE),
        ],
        "BA" => &[Layer::Fill(0x002395), Layer::Diagonal(0.32, 0xFECB00)],
        "RS" => SERBIA,
        "MK" => &[
            Layer::Fill(0xD20000),
            Layer::Disc([0.5, 0.5], 0.22, 0xFFE600),
        ],
        "AL" => &[Layer::Fill(0xE41E20), Layer::Disc([0.5, 0.5], 0.25, BLACK)],
        "GR" => &[
            Layer::H(&[
                (0x0D5EAF, 1.0),
                (WHITE, 1.0),
                (0x0D5EAF, 1.0),
                (WHITE, 1.0),
                (0x0D5EAF, 1.0),
                (WHITE, 1.0),
                (0x0D5EAF, 1.0),
                (WHITE, 1.0),
                (0x0D5EAF, 1.0),
            ]),
            Layer::Sub(
                [0.0, 0.0, 0.37, 0.56],
                &[Layer::Fill(0x0D5EAF), Layer::Cross(0.2, WHITE)],
            ),
        ],
        "BG" => &[Layer::H(&[(WHITE, 1.0), (0x00966E, 1.0), (0xD62612, 1.0)])],
        "RO" => &[Layer::V(&[
            (0x002B7F, 1.0),
            (0xFCD116, 1.0),
            (0xCE1126, 1.0),
        ])],
        "HU" => &[Layer::H(&[(0xCE2939, 1.0), (WHITE, 1.0), (0x477050, 1.0)])],
        "UA" => UKRAINE,
        "BY" => &[
            Layer::H(&[(0xC8313E, 2.0), (0x4AA657, 1.0)]),
            Layer::Rect([0.0, 0.0, 0.12, 1.0], WHITE),
            Layer::Rect([0.03, 0.0, 0.06, 1.0], 0xC8313E),
        ],
        "RU" => RUSSIA,
        "TR" => &[
            Layer::Fill(0xE30A17),
            Layer::Disc([0.38, 0.5], 0.25, WHITE),
            Layer::Disc([0.43, 0.5], 0.2, 0xE30A17),
            Layer::Disc([0.6, 0.5], 0.08, WHITE),
        ],
        "AM" => &[Layer::H(&[
            (0xD90012, 1.0),
            (0x0033A0, 1.0),
            (0xF2A800, 1.0),
        ])],
        "GE" => &[Layer::Fill(WHITE), Layer::Cross(0.2, 0xFF0000)],
        "AZ" => &[
            Layer::H(&[(0x00B5E2, 1.0), (0xEF3340, 1.0), (0x509E2F, 1.0)]),
            Layer::Disc([0.47, 0.5], 0.12, WHITE),
            Layer::Disc([0.5, 0.5], 0.1, 0xEF3340),
        ],
        "KZ" => &[
            Layer::Fill(0x00AFCA),
            Layer::Disc([0.5, 0.45], 0.2, 0xFEC50C),
        ],
        "KG" => &[
            Layer::Fill(0xE8112D),
            Layer::Disc([0.5, 0.5], 0.27, 0xFFEF00),
            Layer::Disc([0.5, 0.5], 0.15, 0xE8112D),
        ],
        "UZ" => &[Layer::H(&[
            (0x0099B5, 10.0),
            (0xCE1126, 1.0),
            (WHITE, 9.0),
            (0xCE1126, 1.0),
            (0x1EB53A, 10.0),
        ])],
        "MN" => &[
            Layer::V(&[(0xC4272F, 1.0), (0x015197, 1.0), (0xC4272F, 1.0)]),
            Layer::Rect([0.12, 0.3, 0.09, 0.4], 0xF9CF02),
        ],
        "CN" => CHINA,
        "HK" => &[
            Layer::Fill(0xDE2910),
            Layer::Disc([0.5, 0.5], 0.27, WHITE),
            Layer::Disc([0.5, 0.5], 0.11, 0xDE2910),
        ],
        "TW" => &[
            Layer::Fill(0xFE0000),
            Layer::Sub(
                [0.0, 0.0, 0.5, 0.5],
                &[Layer::Fill(0x000095), Layer::Disc([0.5, 0.5], 0.28, WHITE)],
            ),
        ],
        "JP" => &[Layer::Fill(WHITE), Layer::Disc([0.5, 0.5], 0.3, 0xBC002D)],
        "KR" => &[
            Layer::Fill(WHITE),
            Layer::Disc([0.5, 0.5], 0.25, 0x0047A0),
            Layer::Disc([0.5, 0.43], 0.18, 0xCD2E3A),
        ],
        "VN" => &[
            Layer::Fill(0xDA251D),
            Layer::Disc([0.5, 0.5], 0.22, 0xFFFF00),
        ],
        "TH" => &[Layer::H(&[
            (0xA51931, 1.0),
            (0xF4F5F8, 1.0),
            (0x2D2A4A, 2.0),
            (0xF4F5F8, 1.0),
            (0xA51931, 1.0),
        ])],
        "LA" => &[
            Layer::H(&[(0xCE1126, 1.0), (0x002868, 2.0), (0xCE1126, 1.0)]),
            Layer::Disc([0.5, 0.5], 0.2, WHITE),
        ],
        "ID" => &[Layer::H(&[(0xFF0000, 1.0), (WHITE, 1.0)])],
        "MY" => MALAYSIA,
        "PH" => PHILIPPINES,
        "IN" => INDIA,
        "IR" => &[Layer::H(&[(0x239F40, 1.0), (WHITE, 1.0), (0xDA0000, 1.0)])],
        "SA" => &[
            Layer::Fill(0x165D31),
            Layer::Rect([0.25, 0.35, 0.5, 0.12], WHITE),
            Layer::Rect([0.3, 0.62, 0.4, 0.05], WHITE),
        ],
        "IL" => &[
            Layer::Fill(WHITE),
            Layer::Rect([0.0, 0.1, 1.0, 0.14], 0x0038B8),
            Layer::Rect([0.0, 0.76, 1.0, 0.14], 0x0038B8),
            Layer::Diamond(0.33, 0x0038B8),
            Layer::Diamond(0.38, WHITE),
        ],
        "ZA" => &[
            Layer::H(&[(0xE03C31, 1.0), (0x007749, 1.0), (0x001489, 1.0)]),
            Layer::Triangle(0.4, 0xFFB81C),
            Layer::Triangle(0.33, BLACK),
        ],
        "NG" => NIGERIA,
        "SO" => &[Layer::Fill(0x4189DD), Layer::Disc([0.5, 0.5], 0.18, WHITE)],
        "IE" => &[Layer::V(&[(0x169B62, 1.0), (WHITE, 1.0), (0xFF883E, 1.0)])],
        "MT" => &[Layer::V(&[(WHITE, 1.0), (0xCF142B, 1.0)])],
        "CU" => &[
            Layer::H(&[
                (0x002A8F, 1.0),
                (WHITE, 1.0),
                (0x002A8F, 1.0),
                (WHITE, 1.0),
                (0x002A8F, 1.0),
            ]),
            Layer::Triangle(0.45, 0xCB1515),
        ],
        "CO" => &[Layer::H(&[
            (0xFCD116, 2.0),
            (0x003893, 1.0),
            (0xCE1126, 1.0),
        ])],
        _ => return None,
    })
}

/// Whether language `code` has a flag of its own (else the neutral tile).
pub fn flag_for(code: &str) -> Option<&'static [Layer]> {
    design(&code.to_ascii_lowercase())
}

/// The 26.3 game's languages (its `pack.mcmeta`), by code: the source of truth for the locale
/// registry the launcher and the website share (`launcher/packages/locales`, ADR 0161).
pub const GAME_LANGUAGES: &[&str] = &[
    "af_za", "ar_sa", "ast_es", "az_az", "ba_ru", "bar", "be_by", "be_latn", "bg_bg", "br_fr",
    "brb", "bs_ba", "ca_es", "cs_cz", "cv_cu", "cy_gb", "da_dk", "de_at", "de_ch", "de_de",
    "el_gr", "en_au", "en_ca", "en_gb", "en_nz", "en_pt", "en_ud", "en_us", "enp", "enws", "eo_uy",
    "es_ar", "es_cl", "es_ec", "es_es", "es_mx", "es_uy", "es_ve", "esan", "et_ee", "eu_es",
    "fa_ir", "fi_fi", "fil_ph", "fo_fo", "fr_ca", "fr_ch", "fr_fr", "fra_de", "fur_it", "fy_nl",
    "ga_ie", "gd_gb", "gl_es", "go_fr", "got_de", "hal_ua", "haw_us", "he_il", "hi_in", "hn_no",
    "hr_hr", "hu_hu", "hy_am", "id_id", "ig_ng", "io_en", "is_is", "isv", "it_it", "ja_jp",
    "jbo_en", "ka_ge", "kk_kz", "kn_in", "ko_kr", "ksh", "kw_gb", "ky_kg", "la_la", "lb_lu",
    "li_li", "lmo", "lo_la", "lol_us", "lt_lt", "lv_lv", "lzh", "mk_mk", "mn_mn", "ms_my", "mt_mt",
    "nah", "nds_de", "nl_be", "nl_nl", "nn_no", "no_no", "oc_fr", "ovd", "pl_pl", "pls", "pt_br",
    "pt_pt", "qcb_es", "qid", "qya_aa", "ro_ro", "rpr", "ru_ru", "ry_ua", "sah_sah", "se_no",
    "sk_sk", "sl_si", "so_so", "sq_al", "sr_cs", "sr_sp", "sv_se", "sxu", "szl", "ta_in", "th_th",
    "tl_ph", "tlh_aa", "tok", "tr_tr", "tt_ru", "tzo_mx", "uk_ua", "uz_uz", "val_es", "vec_it",
    "vi_vn", "vp_vl", "vro", "yi_de", "yo_ng", "zh_cn", "zh_hk", "zh_tw", "zlm_arab",
];

/// Width and height of the box [`shapes`] draws in (3:2, as the language screen shows flags).
pub const SHAPES_SIZE: [f32; 2] = [30.0, 20.0];

/// Flag `code` as plain shapes for an SVG in a [`SHAPES_SIZE`] box: the same flag the game draws,
/// for the launcher and the website (`packages/locales/src/flags.json`). Each shape is
/// `["r", x, y, w, h, colour]`, `["c", cx, cy, r, colour]`, `["p", "x,y x,y …", colour]`,
/// `["l", x1, y1, x2, y2, width, colour]` (a stroke, clipped by the box) or
/// `["s", x, y, w, h, [shapes]]` (another flag in that box, in its own `w` x `h` units).
/// `"f": 1` turns it over (Upside Down English). None for the neutral tile.
pub fn shapes(code: &str) -> Option<serde_json::Value> {
    let layers = flag_for(code)?;
    let [w, h] = SHAPES_SIZE;
    let mut out = serde_json::json!({ "s": layer_shapes(layers, w, h) });
    if code.eq_ignore_ascii_case("en_ud") {
        out["f"] = 1.into();
    }
    Some(out)
}

/// A number with at most two decimals, written as an integer when it is one.
fn num(v: f32) -> serde_json::Value {
    let r = (f64::from(v) * 100.0).round() / 100.0;
    if r.fract() == 0.0 {
        serde_json::json!(r as i64)
    } else {
        serde_json::json!(r)
    }
}

fn colour(c: u32) -> String {
    format!("#{:06x}", c & 0x00FF_FFFF)
}

fn rect(x: f32, y: f32, w: f32, h: f32, c: u32) -> serde_json::Value {
    serde_json::json!(["r", num(x), num(y), num(w), num(h), colour(c)])
}

fn poly(points: &[(f32, f32)], c: u32) -> serde_json::Value {
    let p: Vec<String> = points
        .iter()
        .map(|(x, y)| format!("{},{}", num(*x), num(*y)))
        .collect();
    serde_json::json!(["p", p.join(" "), colour(c)])
}

/// A band `t` thick along a diagonal (from the top left when `down`), drawn long enough for its
/// ends to leave the box, as the sampler covers the whole diagonal.
fn diagonal(down: bool, t: f32, c: u32, w: f32, h: f32) -> serde_json::Value {
    let (y0, y1) = if down { (-h, 2.0 * h) } else { (2.0 * h, -h) };
    serde_json::json!([
        "l",
        num(-w),
        num(y0),
        num(2.0 * w),
        num(y1),
        num(t * h),
        colour(c)
    ])
}

fn layer_shapes(layers: &[Layer], w: f32, h: f32) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for layer in layers {
        match *layer {
            Layer::Fill(c) => out.push(rect(0.0, 0.0, w, h, c)),
            Layer::H(bands) | Layer::V(bands) => {
                let total: f32 = bands.iter().map(|b| b.1).sum();
                let horizontal = matches!(layer, Layer::H(_));
                let mut acc = 0.0;
                for (c, weight) in bands {
                    let (from, to) = (acc / total, (acc + weight) / total);
                    acc += weight;
                    out.push(if horizontal {
                        rect(0.0, from * h, w, (to - from) * h, *c)
                    } else {
                        rect(from * w, 0.0, (to - from) * w, h, *c)
                    });
                }
            }
            Layer::Rect([rx, ry, rw, rh], c) => out.push(rect(rx * w, ry * h, rw * w, rh * h, c)),
            Layer::Disc([cx, cy], r, c) => out.push(serde_json::json!([
                "c",
                num(cx * w),
                num(cy * h),
                num(r * h),
                colour(c)
            ])),
            Layer::Cross(t, c) => {
                let half = t * h / 2.0;
                out.push(rect(w / 2.0 - half, 0.0, 2.0 * half, h, c));
                out.push(rect(0.0, h / 2.0 - half, w, 2.0 * half, c));
            }
            Layer::Plus(t, len, c) => {
                let (half, reach) = (t * h / 2.0, len * h / 2.0);
                out.push(rect(
                    w / 2.0 - half,
                    h / 2.0 - reach,
                    2.0 * half,
                    2.0 * reach,
                    c,
                ));
                out.push(rect(
                    w / 2.0 - reach,
                    h / 2.0 - half,
                    2.0 * reach,
                    2.0 * half,
                    c,
                ));
            }
            Layer::Nordic(cx, t, c) => {
                let half = t * h / 2.0;
                out.push(rect(cx * w - half, 0.0, 2.0 * half, h, c));
                out.push(rect(0.0, h / 2.0 - half, w, 2.0 * half, c));
            }
            Layer::Saltire(t, c) => {
                out.push(diagonal(true, t, c, w, h));
                out.push(diagonal(false, t, c, w, h));
            }
            Layer::Diagonal(t, c) => out.push(diagonal(true, t, c, w, h)),
            Layer::Triangle(depth, c) => {
                out.push(poly(&[(0.0, 0.0), (depth * w, h / 2.0), (0.0, h)], c))
            }
            Layer::Poly(points, c) => {
                let at: Vec<(f32, f32)> = points.iter().map(|p| (p[0] * w, p[1] * h)).collect();
                out.push(poly(&at, c));
            }
            Layer::Diamond(inset, c) => {
                let (hw, hh) = (w / 2.0 - inset * h, h / 2.0 - inset * h);
                out.push(poly(
                    &[
                        (w / 2.0 - hw, h / 2.0),
                        (w / 2.0, h / 2.0 - hh),
                        (w / 2.0 + hw, h / 2.0),
                        (w / 2.0, h / 2.0 + hh),
                    ],
                    c,
                ));
            }
            Layer::Sub([rx, ry, rw, rh], inner) => {
                let (sw, sh) = (rw * w, rh * h);
                out.push(serde_json::json!([
                    "s",
                    num(rx * w),
                    num(ry * h),
                    num(sw),
                    num(sh),
                    layer_shapes(inner, sw, sh)
                ]));
            }
        }
    }
    out
}

/// Every game language's flag shapes (`null` for the neutral tile), as
/// `packages/locales/src/flags.json` holds them.
pub fn shapes_document() -> String {
    let mut flags = serde_json::Map::new();
    for &code in GAME_LANGUAGES {
        flags.insert(
            code.to_owned(),
            shapes(code).unwrap_or(serde_json::Value::Null),
        );
    }
    let doc = serde_json::json!({
        "generated": "rusticmc-companion-core flags.rs; update: RUSTIC_BLESS=1 cargo test -p rusticmc-companion-core flags",
        "size": SHAPES_SIZE,
        "codes": GAME_LANGUAGES,
        "flags": flags,
    });
    let mut text = serde_json::to_string_pretty(&doc).unwrap_or_default();
    text.push('\n');
    text
}

/// The coverage (0-1) of `layer` at the point `(x, y)` of a `w` x `h` flag, its colour.
fn sample(layers: &[Layer], x: f32, y: f32, w: f32, h: f32) -> Option<u32> {
    let mut out = None;
    for layer in layers {
        let (u, v) = (x / w, y / h);
        let hit = match *layer {
            Layer::Fill(c) => Some(c),
            Layer::H(bands) | Layer::V(bands) => {
                let total: f32 = bands.iter().map(|b| b.1).sum();
                let t = if matches!(layer, Layer::H(_)) { v } else { u } * total;
                let mut acc = 0.0;
                bands
                    .iter()
                    .find(|(_, wgt)| {
                        acc += wgt;
                        t < acc
                    })
                    .or(bands.last())
                    .map(|b| b.0)
            }
            Layer::Rect([rx, ry, rw, rh], c) => {
                (u >= rx && u < rx + rw && v >= ry && v < ry + rh).then_some(c)
            }
            Layer::Disc([cx, cy], r, c) => {
                let (dx, dy) = ((u - cx) * w, (v - cy) * h);
                (dx * dx + dy * dy <= (r * h) * (r * h)).then_some(c)
            }
            Layer::Cross(t, c) => {
                let half = t * h / 2.0;
                ((x - w / 2.0).abs() <= half || (y - h / 2.0).abs() <= half).then_some(c)
            }
            Layer::Plus(t, len, c) => {
                let (half, reach) = (t * h / 2.0, len * h / 2.0);
                let (dx, dy) = ((x - w / 2.0).abs(), (y - h / 2.0).abs());
                ((dx <= half && dy <= reach) || (dy <= half && dx <= reach)).then_some(c)
            }
            Layer::Nordic(cx, t, c) => {
                let half = t * h / 2.0;
                ((x - cx * w).abs() <= half || (y - h / 2.0).abs() <= half).then_some(c)
            }
            Layer::Saltire(t, c) => {
                let d1 = (y - h / w * x).abs() * w / (w * w + h * h).sqrt();
                let d2 = (y - (h - h / w * x)).abs() * w / (w * w + h * h).sqrt();
                (d1.min(d2) <= t * h / 2.0).then_some(c)
            }
            Layer::Diagonal(t, c) => {
                let d = (y - h / w * x).abs() * w / (w * w + h * h).sqrt();
                (d <= t * h / 2.0).then_some(c)
            }
            Layer::Triangle(depth, c) => {
                let tip = depth * w;
                (x <= tip * (1.0 - (2.0 * v - 1.0).abs())).then_some(c)
            }
            Layer::Diamond(inset, c) => {
                let (hw, hh) = (w / 2.0 - inset * h, h / 2.0 - inset * h);
                ((x - w / 2.0).abs() / hw.max(0.01) + (y - h / 2.0).abs() / hh.max(0.01) <= 1.0)
                    .then_some(c)
            }
            Layer::Poly(points, c) => {
                // Even-odd ray casting in the unit square.
                let mut inside = false;
                let n = points.len();
                for i in 0..n {
                    let [ax, ay] = points[i];
                    let [bx, by] = points[(i + n - 1) % n];
                    if (ay > v) != (by > v) && u < (bx - ax) * (v - ay) / (by - ay) + ax {
                        inside = !inside;
                    }
                }
                inside.then_some(c)
            }
            Layer::Sub([rx, ry, rw, rh], inner) => {
                if u >= rx && u < rx + rw && v >= ry && v < ry + rh {
                    sample(inner, (u - rx) * w, (v - ry) * h, rw * w, rh * h)
                } else {
                    None
                }
            }
        };
        if hit.is_some() {
            out = hit;
        }
    }
    out
}

/// Flag `code` as a `width` x `height` picture with corners rounded by `radius` pixels, 4x4
/// supersampled; the neutral tile when it has none. Upside Down English's is turned over.
pub fn render(code: &str, width: u32, height: u32, radius: f32) -> Rgba {
    let layers = flag_for(code);
    let flip = code.eq_ignore_ascii_case("en_ud");
    let (w, h) = (width.max(1), height.max(1));
    let mut im = Rgba::new(w, h);
    let n = 4;
    for py in 0..h {
        for px in 0..w {
            let mut acc = [0.0f32; 3];
            let mut cover = 0.0;
            for sy in 0..n {
                for sx in 0..n {
                    let fx = px as f32 + (sx as f32 + 0.5) / n as f32;
                    let mut fy = py as f32 + (sy as f32 + 0.5) / n as f32;
                    if flip {
                        fy = h as f32 - fy;
                    }
                    // Rounded corners.
                    let qx = (fx - w as f32 / 2.0).abs() - (w as f32 / 2.0 - radius);
                    let qy = (fy - h as f32 / 2.0).abs() - (h as f32 / 2.0 - radius);
                    if qx > 0.0 && qy > 0.0 && qx * qx + qy * qy > radius * radius {
                        continue;
                    }
                    let c = match layers {
                        Some(l) => sample(l, fx, fy, w as f32, h as f32).unwrap_or(NEUTRAL),
                        None => NEUTRAL,
                    };
                    let [_, r, g, b] = c.to_be_bytes();
                    acc[0] += r as f32;
                    acc[1] += g as f32;
                    acc[2] += b as f32;
                    cover += 1.0;
                }
            }
            let i = ((py * w + px) * 4) as usize;
            if cover > 0.0 {
                for (c, sum) in acc.iter().enumerate() {
                    im.pixels[i + c] = (sum / cover).round() as u8;
                }
                im.pixels[i + 3] = (cover / (n * n) as f32 * 255.0).round() as u8;
            }
        }
    }
    im
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(im: &Rgba, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * im.width + x) * 4) as usize;
        [
            im.pixels[i],
            im.pixels[i + 1],
            im.pixels[i + 2],
            im.pixels[i + 3],
        ]
    }

    #[test]
    fn every_game_language_has_a_flag_or_the_neutral_tile() {
        // The 26.3 game's languages, all drawable.
        let all = GAME_LANGUAGES;
        let mut known = 0;
        for &code in all {
            let im = render(code, 24, 16, 2.0);
            assert_eq!((im.width, im.height), (24, 16));
            // The middle is drawn, the corner rounded off.
            assert_eq!(px(&im, 12, 8)[3], 255, "{code}");
            assert!(px(&im, 0, 0)[3] < 255, "{code}");
            known += usize::from(flag_for(code).is_some());
        }
        assert_eq!(known, all.len(), "every game language has a picture");
    }

    #[test]
    fn flags_have_their_colours_where_they_belong() {
        let fr = render("fr_fr", 30, 20, 0.0);
        assert_eq!(&px(&fr, 2, 10)[..3], &[0x00, 0x55, 0xA4]);
        assert_eq!(&px(&fr, 15, 10)[..3], &[0xFF, 0xFF, 0xFF]);
        assert_eq!(&px(&fr, 28, 10)[..3], &[0xEF, 0x41, 0x35]);
        let de = render("de_de", 30, 18, 0.0);
        assert_eq!(&px(&de, 15, 1)[..3], &[0x11, 0x11, 0x11]);
        assert_eq!(&px(&de, 15, 16)[..3], &[0xFF, 0xCD, 0x00]);
        let jp = render("ja_jp", 30, 20, 0.0);
        assert_eq!(&px(&jp, 15, 10)[..3], &[0xBC, 0x00, 0x2D]);
        assert_eq!(&px(&jp, 2, 2)[..3], &[0xFF, 0xFF, 0xFF]);
        // The region decides: Canadian French has Canada's flag.
        let ca = render("fr_ca", 32, 16, 0.0);
        assert_eq!(&px(&ca, 2, 8)[..3], &[0xD8, 0x06, 0x21]);
        // Upside Down English is the Union flag turned over.
        let gb = render("en_gb", 32, 16, 0.0);
        let ud = render("en_ud", 32, 16, 0.0);
        assert_eq!(px(&gb, 3, 2), px(&ud, 3, 13));
    }

    #[test]
    fn the_shared_flag_file_is_current() {
        // launcher/packages/locales/src/flags.json, from this crate's folder.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../packages/locales/src/flags.json");
        let want = shapes_document();
        // A source tree without the launcher's packages (an edition source archive) has no copy.
        if !path.parent().is_some_and(std::path::Path::exists) {
            return;
        }
        if std::env::var_os("RUSTIC_BLESS").is_some() {
            std::fs::write(&path, &want).expect("write flags.json");
        }
        let have = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            have == want,
            "{} is stale: run `RUSTIC_BLESS=1 cargo test -p rusticmc-companion-core flags`",
            path.display()
        );
        assert_eq!(GAME_LANGUAGES.len(), 143);
    }

    #[test]
    fn shapes_follow_the_layers() {
        let fr = shapes("fr_fr").expect("France");
        assert_eq!(
            fr["s"][0],
            serde_json::json!(["r", 0, 0, 10, 20, "#0055a4"])
        );
        assert!(shapes("xx_none").is_none());
        // Ido's star is a polygon.
        let io = shapes("io_en").expect("Ido");
        assert_eq!(io["s"][1][0], "p");
        // Old-spelling Indonesian has Indonesia's flag.
        assert_eq!(shapes("qid"), shapes("id_id"));
        assert_eq!(shapes("en_ud").expect("flag")["f"], 1);
        // The canton of Hawaii's flag is the Union flag in its own box.
        let haw = shapes("haw_us").expect("Hawaii");
        let sub = haw["s"].as_array().and_then(|a| a.last()).expect("sub");
        assert_eq!(sub[0], "s");
        assert_eq!(sub[3], 15);
    }
}
