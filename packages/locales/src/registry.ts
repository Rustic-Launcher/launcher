/**
 * Every language the game has (its 143 locales), for the launcher and the website (ADR 0161).
 * The codes are the game's (`de_de`, `pt_br`, `en_ud`...) and come from the companion
 * (`client-mod/rustic-companion/core/src/flags.rs` `GAME_LANGUAGES`, mirrored into `flags.json`
 * with the flags; `registry.test.ts` checks both lists are the same). Names are written here, in
 * each language's own words; the game's own list stays the authority in game.
 *
 * - `tag`: the closest standard BCP 47 tag, for `Intl`, `lang` attributes and i18next. Joke and
 *   constructed variants of English use an English tag.
 * - `tier`: 1 = the ~30 main languages (machine-translated first), 2 = other natural languages and
 *   regional ones (community), 3 = constructed, historical and joke languages.
 * - `fallback`: a game code whose translation reads better than English for this locale
 *   (dialects and close varieties: `bar` → German, `pt_pt` → Brazilian Portuguese).
 * - `verify`: the identity of the code is uncertain (private-use codes); its names are a best
 *   guess, to check against the game's list.
 */

export type Tier = 1 | 2 | 3;

export interface LocaleInfo {
  code: string;
  tag: string;
  /** The language's name in itself. */
  name: string;
  /** Its region, in the language itself (empty when none). */
  region: string;
  /** The language's name in English. */
  english: string;
  tier: Tier;
  dir: 'ltr' | 'rtl';
  fallback?: string;
  verify?: boolean;
}

type Row = [
  code: string,
  tag: string,
  name: string,
  region: string,
  english: string,
  tier: Tier,
  extra?: { rtl?: true; fallback?: string; verify?: true },
];

const ROWS: Row[] = [
  ['af_za', 'af-ZA', 'Afrikaans', 'Suid-Afrika', 'Afrikaans', 2],
  ['ar_sa', 'ar-SA', 'العربية', 'السعودية', 'Arabic', 1, { rtl: true }],
  ['ast_es', 'ast-ES', 'Asturianu', 'Asturies', 'Asturian', 2],
  ['az_az', 'az-AZ', 'Azərbaycanca', 'Azərbaycan', 'Azerbaijani', 2],
  ['ba_ru', 'ba-RU', 'Башҡортса', 'Башҡортостан', 'Bashkir', 2],
  ['bar', 'bar', 'Boarisch', 'Bayern', 'Bavarian', 2, { fallback: 'de_de' }],
  ['be_by', 'be-BY', 'Беларуская', 'Беларусь', 'Belarusian', 2],
  ['be_latn', 'be-Latn', 'Biełaruskaja', 'Łacinka', 'Belarusian (Łacinka)', 2],
  ['bg_bg', 'bg-BG', 'Български', 'България', 'Bulgarian', 2],
  ['br_fr', 'br-FR', 'Brezhoneg', 'Breizh', 'Breton', 2],
  ['brb', 'nl', 'Brabants', 'Brabant', 'Brabantian', 2, { fallback: 'nl_nl' }],
  ['bs_ba', 'bs-BA', 'Bosanski', 'Bosna i Hercegovina', 'Bosnian', 2],
  ['ca_es', 'ca-ES', 'Català', 'Catalunya', 'Catalan', 2],
  ['cs_cz', 'cs-CZ', 'Čeština', 'Česko', 'Czech', 1],
  ['cv_cu', 'cv-RU', 'Чӑвашла', 'Чӑваш Ен', 'Chuvash', 2],
  ['cy_gb', 'cy-GB', 'Cymraeg', 'Cymru', 'Welsh', 2],
  ['da_dk', 'da-DK', 'Dansk', 'Danmark', 'Danish', 1],
  ['de_at', 'de-AT', 'Deutsch', 'Österreich', 'German (Austria)', 1],
  ['de_ch', 'de-CH', 'Deutsch', 'Schweiz', 'German (Switzerland)', 1],
  ['de_de', 'de-DE', 'Deutsch', 'Deutschland', 'German', 1],
  ['el_gr', 'el-GR', 'Ελληνικά', 'Ελλάδα', 'Greek', 1],
  ['en_au', 'en-AU', 'English', 'Australia', 'English (Australia)', 1],
  ['en_ca', 'en-CA', 'English', 'Canada', 'English (Canada)', 1],
  ['en_gb', 'en-GB', 'English', 'United Kingdom', 'English (UK)', 1],
  ['en_nz', 'en-NZ', 'English', 'New Zealand', 'English (New Zealand)', 1],
  ['en_pt', 'en', 'Pirate Speak', 'The Seven Seas', 'Pirate Speak', 3],
  ['en_ud', 'en', 'ɥsᴉlᵷuƎ', 'ɯopᵷuᴉꞰ pǝʇᴉu∩', 'Upside Down English', 3],
  ['en_us', 'en-US', 'English', 'United States', 'English (US)', 1],
  ['enp', 'en-GB', 'Anglish', 'Englaland', 'Anglish', 3],
  ['enws', 'en-GB', 'Shakespearean English', 'Kingdom of England', 'Shakespearean English', 3],
  ['eo_uy', 'eo', 'Esperanto', 'Esperantujo', 'Esperanto', 3],
  ['es_ar', 'es-AR', 'Español', 'Argentina', 'Spanish (Argentina)', 1],
  ['es_cl', 'es-CL', 'Español', 'Chile', 'Spanish (Chile)', 1],
  ['es_ec', 'es-EC', 'Español', 'Ecuador', 'Spanish (Ecuador)', 1],
  ['es_es', 'es-ES', 'Español', 'España', 'Spanish (Spain)', 1],
  ['es_mx', 'es-MX', 'Español', 'México', 'Spanish (Mexico)', 1],
  ['es_uy', 'es-UY', 'Español', 'Uruguay', 'Spanish (Uruguay)', 1],
  ['es_ve', 'es-VE', 'Español', 'Venezuela', 'Spanish (Venezuela)', 1],
  ['esan', 'es-ES', 'Andaluz', 'Andalucía', 'Andalusian', 2, { fallback: 'es_es' }],
  ['et_ee', 'et-EE', 'Eesti', 'Eesti', 'Estonian', 2],
  ['eu_es', 'eu-ES', 'Euskara', 'Euskal Herria', 'Basque', 2],
  ['fa_ir', 'fa-IR', 'فارسی', 'ایران', 'Persian', 2, { rtl: true }],
  ['fi_fi', 'fi-FI', 'Suomi', 'Suomi', 'Finnish', 1],
  ['fil_ph', 'fil-PH', 'Filipino', 'Pilipinas', 'Filipino', 2],
  ['fo_fo', 'fo-FO', 'Føroyskt', 'Føroyar', 'Faroese', 2],
  ['fr_ca', 'fr-CA', 'Français', 'Canada', 'French (Canada)', 1],
  ['fr_ch', 'fr-CH', 'Français', 'Suisse', 'French (Switzerland)', 1],
  ['fr_fr', 'fr-FR', 'Français', 'France', 'French', 1],
  ['fra_de', 'vmf', 'Fränkisch', 'Franken', 'East Franconian', 2, { fallback: 'de_de' }],
  ['fur_it', 'fur-IT', 'Furlan', 'Friûl', 'Friulian', 2],
  ['fy_nl', 'fy-NL', 'Frysk', 'Fryslân', 'West Frisian', 2],
  ['ga_ie', 'ga-IE', 'Gaeilge', 'Éire', 'Irish', 2],
  ['gd_gb', 'gd-GB', 'Gàidhlig', 'Alba', 'Scottish Gaelic', 2],
  ['gl_es', 'gl-ES', 'Galego', 'Galicia', 'Galician', 2],
  ['go_fr', 'fr-FR', 'Galo', 'Bertègn', 'Gallo', 2, { fallback: 'fr_fr', verify: true }],
  ['got_de', 'got', 'Gutisk', 'Gutþiuda', 'Gothic', 3],
  ['hal_ua', 'uk', 'Галицка', 'Галичина', 'Halychian', 2, { fallback: 'uk_ua' }],
  ['haw_us', 'haw-US', 'ʻŌlelo Hawaiʻi', 'Hawaiʻi', 'Hawaiian', 2],
  ['he_il', 'he-IL', 'עברית', 'ישראל', 'Hebrew', 1, { rtl: true }],
  ['hi_in', 'hi-IN', 'हिन्दी', 'भारत', 'Hindi', 1],
  ['hn_no', 'nn-NO', 'Høgnorsk', 'Noreg', 'High Norwegian', 2, { fallback: 'nn_no' }],
  ['hr_hr', 'hr-HR', 'Hrvatski', 'Hrvatska', 'Croatian', 2],
  ['hu_hu', 'hu-HU', 'Magyar', 'Magyarország', 'Hungarian', 1],
  ['hy_am', 'hy-AM', 'Հայերեն', 'Հայաստան', 'Armenian', 2],
  ['id_id', 'id-ID', 'Bahasa Indonesia', 'Indonesia', 'Indonesian', 1],
  ['ig_ng', 'ig-NG', 'Igbo', 'Naịjịrịa', 'Igbo', 2],
  ['io_en', 'io', 'Ido', 'Idolando', 'Ido', 3],
  ['is_is', 'is-IS', 'Íslenska', 'Ísland', 'Icelandic', 2],
  ['isv', 'isv', 'Medžuslovjansky', 'Slovjanstvo', 'Interslavic', 3],
  ['it_it', 'it-IT', 'Italiano', 'Italia', 'Italian', 1],
  ['ja_jp', 'ja-JP', '日本語', '日本', 'Japanese', 1],
  ['jbo_en', 'jbo', 'la .lojban.', 'la jbogu’e', 'Lojban', 3],
  ['ka_ge', 'ka-GE', 'ქართული', 'საქართველო', 'Georgian', 2],
  ['kk_kz', 'kk-KZ', 'Қазақша', 'Қазақстан', 'Kazakh', 2],
  ['kn_in', 'kn-IN', 'ಕನ್ನಡ', 'ಭಾರತ', 'Kannada', 2],
  ['ko_kr', 'ko-KR', '한국어', '대한민국', 'Korean', 1],
  ['ksh', 'ksh', 'Kölsch', 'Kölle', 'Kölsch', 2, { fallback: 'de_de' }],
  ['kw_gb', 'kw-GB', 'Kernewek', 'Kernow', 'Cornish', 2],
  ['ky_kg', 'ky-KG', 'Кыргызча', 'Кыргызстан', 'Kyrgyz', 2],
  ['la_la', 'la', 'Latina', 'Latium', 'Latin', 3],
  ['lb_lu', 'lb-LU', 'Lëtzebuergesch', 'Lëtzebuerg', 'Luxembourgish', 2],
  ['li_li', 'li', 'Limburgs', 'Limburg', 'Limburgish', 2, { fallback: 'nl_nl' }],
  ['lmo', 'lmo', 'Lombard', 'Lombardia', 'Lombard', 2],
  ['lo_la', 'lo-LA', 'ລາວ', 'ລາວ', 'Lao', 2],
  ['lol_us', 'en-US', 'LOLCAT', 'Kingdom of Cats', 'LOLCAT', 3],
  ['lt_lt', 'lt-LT', 'Lietuvių', 'Lietuva', 'Lithuanian', 2],
  ['lv_lv', 'lv-LV', 'Latviešu', 'Latvija', 'Latvian', 2],
  ['lzh', 'lzh', '文言', '華夏', 'Classical Chinese', 3, { fallback: 'zh_tw' }],
  ['mk_mk', 'mk-MK', 'Македонски', 'Северна Македонија', 'Macedonian', 2],
  ['mn_mn', 'mn-MN', 'Монгол', 'Монгол Улс', 'Mongolian', 2],
  ['ms_my', 'ms-MY', 'Bahasa Melayu', 'Malaysia', 'Malay', 2],
  ['mt_mt', 'mt-MT', 'Malti', 'Malta', 'Maltese', 2],
  ['nah', 'nah', 'Nāhuatl', 'Mēxihco', 'Nahuatl', 2, { fallback: 'es_mx' }],
  ['nds_de', 'nds-DE', 'Plattdüütsch', 'Düütschland', 'Low German', 2, { fallback: 'de_de' }],
  ['nl_be', 'nl-BE', 'Nederlands', 'België', 'Dutch (Belgium)', 1],
  ['nl_nl', 'nl-NL', 'Nederlands', 'Nederland', 'Dutch', 1],
  ['nn_no', 'nn-NO', 'Norsk nynorsk', 'Noreg', 'Norwegian Nynorsk', 2, { fallback: 'no_no' }],
  ['no_no', 'nb-NO', 'Norsk bokmål', 'Norge', 'Norwegian Bokmål', 1],
  ['oc_fr', 'oc-FR', 'Occitan', 'Occitània', 'Occitan', 2],
  ['ovd', 'ovd', 'Övdalsk', 'Övdaln', 'Elfdalian', 2, { fallback: 'sv_se' }],
  ['pl_pl', 'pl-PL', 'Polski', 'Polska', 'Polish', 1],
  ['pls', 'pls', 'Ngiwa', 'México', 'Popoloca', 2, { fallback: 'es_mx' }],
  ['pt_br', 'pt-BR', 'Português', 'Brasil', 'Portuguese (Brazil)', 1],
  ['pt_pt', 'pt-PT', 'Português', 'Portugal', 'Portuguese (Portugal)', 1, { fallback: 'pt_br' }],
  [
    'qcb_es',
    'es-ES',
    'qcb',
    'España',
    'Regional language of Spain (qcb)',
    2,
    { fallback: 'es_es', verify: true },
  ],
  ['qid', 'en', 'qid', '', 'Constructed language (qid)', 3, { verify: true }],
  ['qya_aa', 'qya', 'Quenya', 'Arda', 'Quenya', 3],
  ['ro_ro', 'ro-RO', 'Română', 'România', 'Romanian', 1],
  [
    'rpr',
    'ru',
    'Дореформенный русскій',
    'Россійская Имперія',
    'Pre-reform Russian',
    3,
    { fallback: 'ru_ru' },
  ],
  ['ru_ru', 'ru-RU', 'Русский', 'Россия', 'Russian', 1],
  ['ry_ua', 'rue', 'Русиньскый', 'Підкарпатя', 'Rusyn', 2, { fallback: 'uk_ua' }],
  ['sah_sah', 'sah', 'Саха тыла', 'Саха Сирэ', 'Yakut', 2],
  ['se_no', 'se-NO', 'Davvisámegiella', 'Sápmi', 'Northern Sami', 2, { fallback: 'no_no' }],
  ['sk_sk', 'sk-SK', 'Slovenčina', 'Slovensko', 'Slovak', 2],
  ['sl_si', 'sl-SI', 'Slovenščina', 'Slovenija', 'Slovenian', 2],
  ['so_so', 'so-SO', 'Soomaali', 'Soomaaliya', 'Somali', 2],
  ['sq_al', 'sq-AL', 'Shqip', 'Shqipëria', 'Albanian', 2],
  ['sr_cs', 'sr-Latn', 'Srpski', 'Srbija', 'Serbian (Latin)', 2],
  ['sr_sp', 'sr-Cyrl', 'Српски', 'Србија', 'Serbian (Cyrillic)', 2],
  ['sv_se', 'sv-SE', 'Svenska', 'Sverige', 'Swedish', 1],
  ['sxu', 'sxu', 'Säggssch', 'Saggsn', 'Upper Saxon', 2, { fallback: 'de_de' }],
  ['szl', 'szl', 'Ślōnski', 'Gōrny Ślōnsk', 'Silesian', 2, { fallback: 'pl_pl' }],
  ['ta_in', 'ta-IN', 'தமிழ்', 'இந்தியா', 'Tamil', 2],
  ['th_th', 'th-TH', 'ไทย', 'ประเทศไทย', 'Thai', 1],
  ['tl_ph', 'tl-PH', 'Tagalog', 'Pilipinas', 'Tagalog', 2],
  ['tlh_aa', 'tlh', 'tlhIngan Hol', 'Qo’noS', 'Klingon', 3],
  ['tok', 'tok', 'toki pona', 'ma pona', 'Toki Pona', 3],
  ['tr_tr', 'tr-TR', 'Türkçe', 'Türkiye', 'Turkish', 1],
  ['tt_ru', 'tt-RU', 'Татарча', 'Татарстан', 'Tatar', 2],
  ['tzo_mx', 'tzo', 'Bats’i k’op', 'Mejiko', 'Tzotzil', 2, { fallback: 'es_mx' }],
  ['uk_ua', 'uk-UA', 'Українська', 'Україна', 'Ukrainian', 1],
  ['uz_uz', 'uz-UZ', 'Oʻzbekcha', 'Oʻzbekiston', 'Uzbek', 2],
  ['val_es', 'ca-ES-valencia', 'Valencià', 'País Valencià', 'Valencian', 2, { fallback: 'ca_es' }],
  ['vec_it', 'vec-IT', 'Vèneto', 'Itałia', 'Venetian', 2],
  ['vi_vn', 'vi-VN', 'Tiếng Việt', 'Việt Nam', 'Vietnamese', 1],
  ['vp_vl', 'en', 'Viossa', 'Vilant', 'Viossa', 3],
  ['vro', 'vro', 'Võro', 'Võromaa', 'Võro', 2, { fallback: 'et_ee' }],
  ['yi_de', 'yi', 'ייִדיש', 'אייראָפּע', 'Yiddish', 2, { rtl: true }],
  ['yo_ng', 'yo-NG', 'Yorùbá', 'Nàìjíríà', 'Yoruba', 2],
  ['zh_cn', 'zh-CN', '简体中文', '中国大陆', 'Chinese (Simplified)', 1],
  ['zh_hk', 'zh-HK', '繁體中文', '香港特別行政區', 'Chinese (Hong Kong)', 2, { fallback: 'zh_tw' }],
  ['zh_tw', 'zh-TW', '繁體中文', '台灣', 'Chinese (Traditional)', 1],
  [
    'zlm_arab',
    'ms-Arab',
    'بهاس ملايو',
    'مليسيا',
    'Malay (Jawi)',
    2,
    { rtl: true, fallback: 'ms_my' },
  ],
];

export const LOCALES: readonly LocaleInfo[] = ROWS.map(
  ([code, tag, name, region, english, tier, extra]) => ({
    code,
    tag,
    name,
    region,
    english,
    tier,
    dir: extra?.rtl ? 'rtl' : 'ltr',
    ...(extra?.fallback ? { fallback: extra.fallback } : {}),
    ...(extra?.verify ? { verify: true } : {}),
  }),
);

export { COMPLETE, type CompleteBundle } from './complete';

/** The game code a language (or bundle) means when nothing more precise is known. */
export const PRIMARY: Record<string, string> = {
  en: 'en_us',
  fr: 'fr_fr',
  de: 'de_de',
  es: 'es_es',
  it: 'it_it',
  pt: 'pt_br',
  'pt-BR': 'pt_br',
  'pt-PT': 'pt_pt',
  pl: 'pl_pl',
  nl: 'nl_nl',
  sv: 'sv_se',
  nb: 'no_no',
  no: 'no_no',
  nn: 'nn_no',
  zh: 'zh_cn',
  'zh-Hans': 'zh_cn',
  'zh-Hant': 'zh_tw',
  sr: 'sr_sp',
  ca: 'ca_es',
  ms: 'ms_my',
  ru: 'ru_ru',
  uk: 'uk_ua',
};
