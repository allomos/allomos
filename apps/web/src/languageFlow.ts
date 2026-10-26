// The searchable language menu's list (Task-A05-board831, approver 2026-10-26), kept apart from
// React so Node can test it. Every language with an ISO 639-1 code (an agent can write in each),
// plus a few regional variants people ask for; names come from the system's own language data
// (`Intl.DisplayNames`), so nothing is translated by hand. The interface languages come first,
// then every other language A to Z by English name.

/** ISO 639-1, without `zh` and `pt` (offered as their variants below). */
const ISO_639_1 =
  "aa ab ae af ak am an ar as av ay az ba be bg bi bm bn bo br bs ca ce ch co cr cs cu cv cy da de dv dz ee el en eo es et eu fa ff fi fj fo fr fy ga gd gl gn gu gv ha he hi ho hr ht hu hy hz ia id ie ig ii ik io is it iu ja jv ka kg ki kj kk kl km kn ko kr ks ku kv kw ky la lb lg li ln lo lt lu lv mg mh mi mk ml mn mr ms mt my na nb nd ne ng nl nn no nr nv ny oc oj om or os pa pi pl ps qu rm rn ro ru rw sa sc sd se sg si sk sl sm sn so sq sr ss st su sv sw ta te tg th ti tk tl tn to tr ts tt tw ty ug uk ur uz ve vi vo wa wo xh yi yo za zu";

/** Regional variants and a few languages without a two-letter code. */
const MORE = ["zh-CN", "zh-TW", "pt-BR", "pt-PT", "yue", "fil"];

/** English names, for a system whose language data lacks some (a web view may know only a few). */
const ENGLISH: Record<string, string> = {
  aa: "Afar", ab: "Abkhazian", ae: "Avestan", af: "Afrikaans", ak: "Akan", am: "Amharic", an: "Aragonese", ar: "Arabic",
  as: "Assamese", av: "Avaric", ay: "Aymara", az: "Azerbaijani", ba: "Bashkir", be: "Belarusian", bg: "Bulgarian",
  bi: "Bislama", bm: "Bambara", bn: "Bangla", bo: "Tibetan", br: "Breton", bs: "Bosnian", ca: "Catalan", ce: "Chechen",
  ch: "Chamorro", co: "Corsican", cr: "Cree", cs: "Czech", cu: "Church Slavic", cv: "Chuvash", cy: "Welsh", da: "Danish",
  de: "German", dv: "Divehi", dz: "Dzongkha", ee: "Ewe", el: "Greek", en: "English", eo: "Esperanto", es: "Spanish",
  et: "Estonian", eu: "Basque", fa: "Persian", ff: "Fula", fi: "Finnish", fj: "Fijian", fo: "Faroese", fr: "French",
  fy: "Western Frisian", ga: "Irish", gd: "Scottish Gaelic", gl: "Galician", gn: "Guarani", gu: "Gujarati", gv: "Manx",
  ha: "Hausa", he: "Hebrew", hi: "Hindi", ho: "Hiri Motu", hr: "Croatian", ht: "Haitian Creole", hu: "Hungarian",
  hy: "Armenian", hz: "Herero", ia: "Interlingua", id: "Indonesian", ie: "Interlingue", ig: "Igbo", ii: "Sichuan Yi",
  ik: "Inupiaq", io: "Ido", is: "Icelandic", it: "Italian", iu: "Inuktitut", ja: "Japanese", jv: "Javanese",
  ka: "Georgian", kg: "Kongo", ki: "Kikuyu", kj: "Kuanyama", kk: "Kazakh", kl: "Kalaallisut", km: "Khmer", kn: "Kannada",
  ko: "Korean", kr: "Kanuri", ks: "Kashmiri", ku: "Kurdish", kv: "Komi", kw: "Cornish", ky: "Kyrgyz", la: "Latin",
  lb: "Luxembourgish", lg: "Ganda", li: "Limburgish", ln: "Lingala", lo: "Lao", lt: "Lithuanian", lu: "Luba-Katanga",
  lv: "Latvian", mg: "Malagasy", mh: "Marshallese", mi: "Māori", mk: "Macedonian", ml: "Malayalam", mn: "Mongolian",
  mr: "Marathi", ms: "Malay", mt: "Maltese", my: "Burmese", na: "Nauru", nb: "Norwegian Bokmål", nd: "North Ndebele",
  ne: "Nepali", ng: "Ndonga", nl: "Dutch", nn: "Norwegian Nynorsk", no: "Norwegian", nr: "South Ndebele", nv: "Navajo",
  ny: "Nyanja", oc: "Occitan", oj: "Ojibwa", om: "Oromo", or: "Odia", os: "Ossetic", pa: "Punjabi", pi: "Pali",
  pl: "Polish", ps: "Pashto", qu: "Quechua", rm: "Romansh", rn: "Rundi", ro: "Romanian", ru: "Russian",
  rw: "Kinyarwanda", sa: "Sanskrit", sc: "Sardinian", sd: "Sindhi", se: "Northern Sami", sg: "Sango", si: "Sinhala",
  sk: "Slovak", sl: "Slovenian", sm: "Samoan", sn: "Shona", so: "Somali", sq: "Albanian", sr: "Serbian", ss: "Swati",
  st: "Southern Sotho", su: "Sundanese", sv: "Swedish", sw: "Swahili", ta: "Tamil", te: "Telugu", tg: "Tajik", th: "Thai",
  ti: "Tigrinya", tk: "Turkmen", tl: "Tagalog", tn: "Tswana", to: "Tongan", tr: "Turkish", ts: "Tsonga", tt: "Tatar",
  tw: "Twi", ty: "Tahitian", ug: "Uyghur", uk: "Ukrainian", ur: "Urdu", uz: "Uzbek", ve: "Venda", vi: "Vietnamese",
  vo: "Volapük", wa: "Walloon", wo: "Wolof", xh: "Xhosa", yi: "Yiddish", yo: "Yoruba", za: "Zhuang", zu: "Zulu",
  "zh-CN": "Chinese (Simplified)", "zh-TW": "Chinese (Traditional)", "pt-BR": "Portuguese (Brazil)",
  "pt-PT": "Portuguese (Portugal)", yue: "Cantonese", fil: "Filipino",
};

/** Own names that read better than the system's ("中文（中国）"), as the interface names them. */
const OWN: Record<string, string> = { "zh-CN": "简体中文", "zh-TW": "繁體中文" };

/** The interface languages, first in the list (as `LANGUAGES` in i18n). */
export const INTERFACE_CODES = ["en", "zh-CN", "ja"] as const;

export const LANGUAGE_CODES: readonly string[] = [...ISO_639_1.split(" "), ...MORE];

export interface LanguageOption {
  code: string;
  /** The language's name in itself ("Tiếng Việt"). */
  own: string;
  /** Its English name ("Vietnamese"). */
  english: string;
}

/** `code`'s name in `locale`; null when the system does not know it. */
function displayName(code: string, locale: string): string | null {
  try {
    const names = new Intl.DisplayNames([locale], { type: "language", fallback: "none" });
    // A system without that language's data answers in another one (its own): not a name in it.
    const base = (tag: string) => tag.split("-")[0].toLowerCase();
    if (base(names.resolvedOptions().locale) !== base(locale)) return null;
    const name = names.of(code);
    return name && name.toLowerCase() !== code.toLowerCase() ? name : null;
  } catch {
    return null;
  }
}

/** One language's names; the code stands in for a name the system does not know. */
export function languageOption(code: string): LanguageOption {
  const english = ENGLISH[code] ?? displayName(code, "en") ?? code;
  const own = OWN[code] ?? displayName(code, code) ?? english;
  // A name starts with a capital where the script has one ("español" → "Español").
  return { code, own: own.charAt(0).toLocaleUpperCase(code) + own.slice(1), english };
}

let cached: LanguageOption[] | null = null;

/** Every language, interface languages first, then the rest A to Z by English name. */
export function allLanguages(): LanguageOption[] {
  if (cached) return cached;
  const options = LANGUAGE_CODES.map(languageOption);
  const first = INTERFACE_CODES.map((c) => options.find((o) => o.code === c)!).filter(Boolean);
  const rest = options
    .filter((o) => !(INTERFACE_CODES as readonly string[]).includes(o.code))
    .sort((a, b) => a.english.localeCompare(b.english, "en"));
  cached = [...first, ...rest];
  return cached;
}

/** Lowercase, without accents: "Tiếng" and "tieng" are the same. */
export function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .trim();
}

/** The languages whose own or English name contains `query`, or whose code is exactly it; the
 * whole list for an empty query. The order stays as in the list. */
export function searchLanguages(options: readonly LanguageOption[], query: string): LanguageOption[] {
  const q = fold(query);
  if (!q) return [...options];
  return options.filter((o) => o.code.toLowerCase() === q || fold(o.own).includes(q) || fold(o.english).includes(q));
}

/** What the field shows for `code`: its own name, or the code when it is not in the list. */
export function languageLabel(code: string): string {
  return allLanguages().find((o) => o.code === code)?.own ?? languageOption(code).own;
}
