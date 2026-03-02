/**
 * @fileoverview Formatter utilities for generating uniquely-named output paths and writing files natively.
 */
import { writeFile } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { basename } from 'node:path';

const MAX_CODE_POINT = 0x10FFFF;
/** Unicode surrogate range — not valid scalar values; String.fromCodePoint rejects them. */
const SURROGATE_MIN = 0xD800;
const SURROGATE_MAX = 0xDFFF;

/**
 * HTML5 named character references — a comprehensive subset from the spec
 * (https://html.spec.whatwg.org/multipage/named-characters.html).
 * Entity names are case-sensitive exactly as the spec defines them.
 * Values are the decoded Unicode string (one or two code points).
 * Unrecognised names are left as-is by decodeEntities.
 */
const HTML_ENTITIES = {
    // ── XML / HTML core ────────────────────────────────────────────────────
    amp: '&', AMP: '&', lt: '<', LT: '<', gt: '>', GT: '>',
    quot: '"', QUOT: '"', apos: "'",

    // ── Latin-1 Supplement (U+00A0–U+00FF) ────────────────────────────────
    nbsp: '\u00A0', iexcl: '\u00A1', cent: '\u00A2', pound: '\u00A3',
    curren: '\u00A4', yen: '\u00A5', brvbar: '\u00A6', sect: '\u00A7',
    uml: '\u00A8', copy: '\u00A9', COPY: '\u00A9', ordf: '\u00AA',
    laquo: '\u00AB', not: '\u00AC', shy: '\u00AD', reg: '\u00AE',
    REG: '\u00AE', macr: '\u00AF', deg: '\u00B0', plusmn: '\u00B1',
    sup2: '\u00B2', sup3: '\u00B3', acute: '\u00B4', micro: '\u00B5',
    para: '\u00B6', middot: '\u00B7', cedil: '\u00B8', sup1: '\u00B9',
    ordm: '\u00BA', raquo: '\u00BB', frac14: '\u00BC', frac12: '\u00BD',
    frac34: '\u00BE', iquest: '\u00BF',
    Agrave: '\u00C0', Aacute: '\u00C1', Acirc: '\u00C2', Atilde: '\u00C3',
    Auml: '\u00C4', Aring: '\u00C5', AElig: '\u00C6', Ccedil: '\u00C7',
    Egrave: '\u00C8', Eacute: '\u00C9', Ecirc: '\u00CA', Euml: '\u00CB',
    Igrave: '\u00CC', Iacute: '\u00CD', Icirc: '\u00CE', Iuml: '\u00CF',
    ETH: '\u00D0', Ntilde: '\u00D1', Ograve: '\u00D2', Oacute: '\u00D3',
    Ocirc: '\u00D4', Otilde: '\u00D5', Ouml: '\u00D6', times: '\u00D7',
    Oslash: '\u00D8', Ugrave: '\u00D9', Uacute: '\u00DA', Ucirc: '\u00DB',
    Uuml: '\u00DC', Yacute: '\u00DD', THORN: '\u00DE', szlig: '\u00DF',
    agrave: '\u00E0', aacute: '\u00E1', acirc: '\u00E2', atilde: '\u00E3',
    auml: '\u00E4', aring: '\u00E5', aelig: '\u00E6', ccedil: '\u00E7',
    egrave: '\u00E8', eacute: '\u00E9', ecirc: '\u00EA', euml: '\u00EB',
    igrave: '\u00EC', iacute: '\u00ED', icirc: '\u00EE', iuml: '\u00EF',
    eth: '\u00F0', ntilde: '\u00F1', ograve: '\u00F2', oacute: '\u00F3',
    ocirc: '\u00F4', otilde: '\u00F5', ouml: '\u00F6', divide: '\u00F7',
    oslash: '\u00F8', ugrave: '\u00F9', uacute: '\u00FA', ucirc: '\u00FB',
    uuml: '\u00FC', yacute: '\u00FD', thorn: '\u00FE', yuml: '\u00FF',

    // ── Latin Extended-A (U+0100–U+017E) ──────────────────────────────────
    Amacr: '\u0100', amacr: '\u0101', Abreve: '\u0102', abreve: '\u0103',
    Aogon: '\u0104', aogon: '\u0105', Cacute: '\u0106', cacute: '\u0107',
    Ccirc: '\u0108', ccirc: '\u0109', Cdot: '\u010A', cdot: '\u010B',
    Ccaron: '\u010C', ccaron: '\u010D', Dcaron: '\u010E', dcaron: '\u010F',
    Dstrok: '\u0110', dstrok: '\u0111', Emacr: '\u0112', emacr: '\u0113',
    Ebreve: '\u0114', ebreve: '\u0115', Edot: '\u0116', edot: '\u0117',
    Eogon: '\u0118', eogon: '\u0119', Ecaron: '\u011A', ecaron: '\u011B',
    Gcirc: '\u011C', gcirc: '\u011D', Gbreve: '\u011E', gbreve: '\u011F',
    Gdot: '\u0120', gdot: '\u0121', Gcedil: '\u0122', gcedil: '\u0123',
    Hcirc: '\u0124', hcirc: '\u0125', Hstrok: '\u0126', hstrok: '\u0127',
    Itilde: '\u0128', itilde: '\u0129', Imacr: '\u012A', imacr: '\u012B',
    Ibreve: '\u012C', ibreve: '\u012D', Iogon: '\u012E', iogon: '\u012F',
    Idot: '\u0130', imath: '\u0131', IJlig: '\u0132', ijlig: '\u0133',
    Jcirc: '\u0134', jcirc: '\u0135', Kcedil: '\u0136', kcedil: '\u0137',
    kgreen: '\u0138', Lacute: '\u0139', lacute: '\u013A', Lcedil: '\u013B',
    lcedil: '\u013C', Lcaron: '\u013D', lcaron: '\u013E', Lmidot: '\u013F',
    lmidot: '\u0140', Lstrok: '\u0141', lstrok: '\u0142', Nacute: '\u0143',
    nacute: '\u0144', Ncedil: '\u0145', ncedil: '\u0146', Ncaron: '\u0147',
    ncaron: '\u0148', napos: '\u0149', ENG: '\u014A', eng: '\u014B',
    Omacr: '\u014C', omacr: '\u014D', Obreve: '\u014E', obreve: '\u014F',
    Odblac: '\u0150', odblac: '\u0151', OElig: '\u0152', oelig: '\u0153',
    Racute: '\u0154', racute: '\u0155', Rcedil: '\u0156', rcedil: '\u0157',
    Rcaron: '\u0158', rcaron: '\u0159', Sacute: '\u015A', sacute: '\u015B',
    Scirc: '\u015C', scirc: '\u015D', Scedil: '\u015E', scedil: '\u015F',
    Scaron: '\u0160', scaron: '\u0161', Tcedil: '\u0162', tcedil: '\u0163',
    Tcaron: '\u0164', tcaron: '\u0165', Tstrok: '\u0166', tstrok: '\u0167',
    Utilde: '\u0168', utilde: '\u0169', Umacr: '\u016A', umacr: '\u016B',
    Ubreve: '\u016C', ubreve: '\u016D', Uring: '\u016E', uring: '\u016F',
    Udblac: '\u0170', udblac: '\u0171', Uogon: '\u0172', uogon: '\u0173',
    Wcirc: '\u0174', wcirc: '\u0175', Ycirc: '\u0176', ycirc: '\u0177',
    Yuml: '\u0178', Zacute: '\u0179', zacute: '\u017A', Zdot: '\u017B',
    zdot: '\u017C', Zcaron: '\u017D', zcaron: '\u017E',

    // ── Latin Extended-B ───────────────────────────────────────────────────
    fnof: '\u0192', imped: '\u01B5',

    // ── Spacing Modifier Letters ───────────────────────────────────────────
    circ: '\u02C6', caron: '\u02C7', breve: '\u02D8', dot: '\u02D9',
    ring: '\u02DA', ogon: '\u02DB', tilde: '\u02DC', dblac: '\u02DD',

    // ── Greek (U+0391–U+03F6) ─────────────────────────────────────────────
    Alpha: '\u0391', Beta: '\u0392', Gamma: '\u0393', Delta: '\u0394',
    Epsilon: '\u0395', Zeta: '\u0396', Eta: '\u0397', Theta: '\u0398',
    Iota: '\u0399', Kappa: '\u039A', Lambda: '\u039B', Mu: '\u039C',
    Nu: '\u039D', Xi: '\u039E', Omicron: '\u039F', Pi: '\u03A0',
    Rho: '\u03A1', Sigma: '\u03A3', Tau: '\u03A4', Upsilon: '\u03A5',
    Phi: '\u03A6', Chi: '\u03A7', Psi: '\u03A8', Omega: '\u03A9',
    alpha: '\u03B1', beta: '\u03B2', gamma: '\u03B3', delta: '\u03B4',
    epsilon: '\u03B5', zeta: '\u03B6', eta: '\u03B7', theta: '\u03B8',
    iota: '\u03B9', kappa: '\u03BA', lambda: '\u03BB', mu: '\u03BC',
    nu: '\u03BD', xi: '\u03BE', omicron: '\u03BF', pi: '\u03C0',
    rho: '\u03C1', sigmaf: '\u03C2', sigma: '\u03C3', tau: '\u03C4',
    upsilon: '\u03C5', phi: '\u03C6', chi: '\u03C7', psi: '\u03C8',
    omega: '\u03C9', thetasym: '\u03D1', upsih: '\u03D2', straightphi: '\u03D5',
    piv: '\u03D6', Gammad: '\u03DC', gammad: '\u03DD', varkappa: '\u03F0',
    varrho: '\u03F1', straightepsilon: '\u03F5', backepsilon: '\u03F6',

    // ── General Punctuation (U+2000–U+206F) ───────────────────────────────
    ensp: '\u2002', emsp: '\u2003', emsp13: '\u2004', emsp14: '\u2005',
    numsp: '\u2007', puncsp: '\u2008', thinsp: '\u2009', hairsp: '\u200A',
    zwnj: '\u200C', zwj: '\u200D', lrm: '\u200E', rlm: '\u200F',
    ndash: '\u2013', mdash: '\u2014', horbar: '\u2015', Verbar: '\u2016',
    lsquo: '\u2018', rsquo: '\u2019', sbquo: '\u201A', ldquo: '\u201C',
    rdquo: '\u201D', bdquo: '\u201E', dagger: '\u2020', Dagger: '\u2021',
    bull: '\u2022', nldr: '\u2025', hellip: '\u2026', permil: '\u2030',
    pertenk: '\u2031', prime: '\u2032', Prime: '\u2033', tprime: '\u2034',
    bprime: '\u2035', lsaquo: '\u2039', rsaquo: '\u203A', oline: '\u203E',
    caret: '\u2041', hybull: '\u2043', frasl: '\u2044', bsemi: '\u204F',
    qprime: '\u2057', MediumSpace: '\u205F',

    // ── Currency Symbols (U+20AC) ─────────────────────────────────────────
    euro: '\u20AC',

    // ── Combining Diacritical Marks for Symbols (U+20D0–U+20FF) ──────────
    tdot: '\u20DB', DotDot: '\u20DC',

    // ── Letterlike Symbols (U+2100–U+214F) ────────────────────────────────
    incare: '\u2105', Copf: '\u2102', weierp: '\u2118', image: '\u2111',
    real: '\u211C', trade: '\u2122', TRADE: '\u2122', alefsym: '\u2135',
    beth: '\u2136', gimel: '\u2137', daleth: '\u2138',

    // ── Number Forms (U+2153–U+215E) ──────────────────────────────────────
    frac13: '\u2153', frac23: '\u2154', frac15: '\u2155', frac25: '\u2156',
    frac35: '\u2157', frac45: '\u2158', frac16: '\u2159', frac56: '\u215A',
    frac18: '\u215B', frac38: '\u215C', frac58: '\u215D', frac78: '\u215E',

    // ── Arrows (U+2190–U+21FF) ────────────────────────────────────────────
    larr: '\u2190', uarr: '\u2191', rarr: '\u2192', darr: '\u2193',
    harr: '\u2194', varr: '\u2195', nwarr: '\u2196', nearr: '\u2197',
    searr: '\u2198', swarr: '\u2199', nlarr: '\u219A', nrarr: '\u219B',
    rarrw: '\u219D', Larr: '\u219E', Uarr: '\u219F', Rarr: '\u21A0',
    Darr: '\u21A1', larrtl: '\u21A2', rarrtl: '\u21A3', larrhk: '\u21A9',
    rarrhk: '\u21AA', larrlp: '\u21AB', rarrlp: '\u21AC', harrw: '\u21AD',
    nharr: '\u21AE', lsh: '\u21B0', rsh: '\u21B1', ldsh: '\u21B2',
    rdsh: '\u21B3', crarr: '\u21B5', cularr: '\u21B6', curarr: '\u21B7',
    olarr: '\u21BA', orarr: '\u21BB', lharu: '\u21BC', lhard: '\u21BD',
    uharr: '\u21BE', uharl: '\u21BF', rharu: '\u21C0', rhard: '\u21C1',
    dharr: '\u21C2', dharl: '\u21C3', rlarr: '\u21C4', udarr: '\u21C5',
    lrarr: '\u21C6', llarr: '\u21C7', uuarr: '\u21C8', rrarr: '\u21C9',
    ddarr: '\u21CA', lrhar: '\u21CB', rlhar: '\u21CC', nlArr: '\u21CD',
    nhArr: '\u21CE', nrArr: '\u21CF', lArr: '\u21D0', uArr: '\u21D1',
    rArr: '\u21D2', dArr: '\u21D3', hArr: '\u21D4', vArr: '\u21D5',
    nwArr: '\u21D6', neArr: '\u21D7', seArr: '\u21D8', swArr: '\u21D9',
    lAarr: '\u21DA', rAarr: '\u21DB', zigrarr: '\u21DD', larrb: '\u21E4',
    rarrb: '\u21E5', duarr: '\u21F5', loarr: '\u21FD', roarr: '\u21FE',
    hoarr: '\u21FF',

    // ── Mathematical Operators (U+2200–U+22FF) ────────────────────────────
    forall: '\u2200', comp: '\u2201', part: '\u2202', exist: '\u2203',
    nexist: '\u2204', empty: '\u2205', emptyset: '\u2205', nabla: '\u2207',
    isin: '\u2208', isinv: '\u2208', notin: '\u2209', ni: '\u220B',
    notni: '\u220C', prod: '\u220F', coprod: '\u2210', sum: '\u2211',
    minus: '\u2212', mnplus: '\u2213', plusdo: '\u2214', setmn: '\u2216',
    lowast: '\u2217', compfn: '\u2218', radic: '\u221A', prop: '\u221D',
    infin: '\u221E', ang: '\u2220', angmsd: '\u2221', angsph: '\u2222',
    mid: '\u2223', nmid: '\u2224', par: '\u2225', npar: '\u2226',
    and: '\u2227', or: '\u2228', cap: '\u2229', cup: '\u222A',
    int: '\u222B', Int: '\u222C', tint: '\u222D', conint: '\u222E',
    Conint: '\u222F', Cconint: '\u2230', cwint: '\u2231', cwconint: '\u2232',
    awconint: '\u2233', there4: '\u2234', because: '\u2235', ratio: '\u2236',
    Colon: '\u2237', minusd: '\u2238', mDDot: '\u223A', homtht: '\u223B',
    sim: '\u223C', bsim: '\u223D', ac: '\u223E', mstpos: '\u223F',
    cong: '\u2245', asymp: '\u2248', bump: '\u224E', bumpe: '\u224F',
    esdot: '\u2250', eDot: '\u2251', efDot: '\u2252', erDot: '\u2253',
    colone: '\u2254', ecolon: '\u2255', ecir: '\u2256', cire: '\u2257',
    wedgeq: '\u2259', equest: '\u225F', ne: '\u2260', equiv: '\u2261',
    nequiv: '\u2262', le: '\u2264', ge: '\u2265', lE: '\u2266',
    gE: '\u2267', lnE: '\u2268', gnE: '\u2269', Lt: '\u226A', Gt: '\u226B',
    twixt: '\u226C', NotCupCap: '\u226D', nlt: '\u226E', ngt: '\u226F',
    nle: '\u2270', nge: '\u2271', lsim: '\u2272', gsim: '\u2273',
    nlsim: '\u2274', ngsim: '\u2275', lg: '\u2276', gl: '\u2277',
    ntlg: '\u2278', ntgl: '\u2279', pr: '\u227A', sc: '\u227B',
    prcue: '\u227C', sccue: '\u227D', prsim: '\u227E', scsim: '\u227F',
    npr: '\u2280', nsc: '\u2281', sub: '\u2282', sup: '\u2283',
    nsub: '\u2284', nsup: '\u2285', sube: '\u2286', supe: '\u2287',
    nsube: '\u2288', nsupe: '\u2289', subne: '\u228A', supne: '\u228B',
    uplus: '\u228E', sqsub: '\u228F', sqsup: '\u2290', sqsube: '\u2291',
    sqsupe: '\u2292', sqcap: '\u2293', sqcup: '\u2294', oplus: '\u2295',
    ominus: '\u2296', otimes: '\u2297', osol: '\u2298', odot: '\u2299',
    ocir: '\u229A', oast: '\u229B', odash: '\u229D', plusb: '\u229E',
    minusb: '\u229F', timesb: '\u22A0', sdotb: '\u22A1', vdash: '\u22A2',
    dashv: '\u22A3', top: '\u22A4', perp: '\u22A5', models: '\u22A7',
    vDash: '\u22A8', Vdash: '\u22A9', VDash: '\u22AB', nvdash: '\u22AC',
    nvDash: '\u22AD', nVdash: '\u22AE', nVDash: '\u22AF', prurel: '\u22B0',
    vltri: '\u22B2', vrtri: '\u22B3', ltrie: '\u22B4', rtrie: '\u22B5',
    origof: '\u22B6', imof: '\u22B7', mumap: '\u22B8', hercon: '\u22B9',
    intcal: '\u22BA', veebar: '\u22BB', barvee: '\u22BD', angrtvb: '\u22BE',
    lrtri: '\u22BF', xwedge: '\u22C0', xvee: '\u22C1', xcap: '\u22C2',
    xcup: '\u22C3', diamond: '\u22C4', sdot: '\u22C5', sstarf: '\u22C6',
    divonx: '\u22C7', bowtie: '\u22C8', ltimes: '\u22C9', rtimes: '\u22CA',
    lthree: '\u22CB', rthree: '\u22CC', bsime: '\u22CD', cuvee: '\u22CE',
    cuwed: '\u22CF', Sub: '\u22D0', Sup: '\u22D1', Cap: '\u22D2',
    Cup: '\u22D3', fork: '\u22D4', epar: '\u22D5', ltdot: '\u22D6',
    gtdot: '\u22D7', Ll: '\u22D8', Gg: '\u22D9', leg: '\u22DA',
    gel: '\u22DB', cuepr: '\u22DE', cuesc: '\u22DF', nprcue: '\u22E0',
    nsccue: '\u22E1', nsqsube: '\u22E2', nsqsupe: '\u22E3', lnsim: '\u22E6',
    gnsim: '\u22E7', prnsim: '\u22E8', scnsim: '\u22E9', nltri: '\u22EA',
    nrtri: '\u22EB', nltrie: '\u22EC', nrtrie: '\u22ED', vellip: '\u22EE',
    ctdot: '\u22EF', utdot: '\u22F0', dtdot: '\u22F1', disin: '\u22F2',
    isinsv: '\u22F3', isins: '\u22F4', isindot: '\u22F5', notinvc: '\u22F6',
    notinvb: '\u22F7', isinE: '\u22F9', nisd: '\u22FA', xnis: '\u22FB',
    nis: '\u22FC', notnivc: '\u22FD', notnivb: '\u22FE',

    // ── Miscellaneous Technical (U+2300–U+23FF) ───────────────────────────
    lceil: '\u2308', rceil: '\u2309', lfloor: '\u230A', rfloor: '\u230B',
    drcrop: '\u230C', dlcrop: '\u230D', urcrop: '\u230E', ulcrop: '\u230F',
    bnot: '\u2310', profline: '\u2312', profsurf: '\u2313', telrec: '\u2315',
    target: '\u2316', ulcorn: '\u231C', urcorn: '\u231D', dlcorn: '\u231E',
    drcorn: '\u231F', frown: '\u2322', smile: '\u2323', cylcty: '\u232D',
    ovbar: '\u233D', solbar: '\u233F', angzarr: '\u237C', lmoust: '\u23B0',
    rmoust: '\u23B1', tbrk: '\u23B4', bbrk: '\u23B5', bbrktbrk: '\u23B6',
    OverParenthesis: '\u23DC', UnderParenthesis: '\u23DD',
    OverBrace: '\u23DE', UnderBrace: '\u23DF', trpezium: '\u23E2',
    elinters: '\u23E7',

    // ── Enclosed Alphanumerics / Control Pictures ─────────────────────────
    blank: '\u2423',

    // ── Box Drawing (U+2500–U+256C) ───────────────────────────────────────
    boxh: '\u2500', boxv: '\u2502', boxdr: '\u250C', boxdl: '\u2510',
    boxur: '\u2514', boxul: '\u2518', boxvr: '\u251C', boxvl: '\u2524',
    boxhd: '\u252C', boxhu: '\u2534', boxvh: '\u253C', boxH: '\u2550',
    boxV: '\u2551', boxdR: '\u2552', boxDr: '\u2553', boxDR: '\u2554',
    boxdL: '\u2555', boxDl: '\u2556', boxDL: '\u2557', boxuR: '\u2558',
    boxUr: '\u2559', boxUR: '\u255A', boxuL: '\u255B', boxUl: '\u255C',
    boxUL: '\u255D', boxvR: '\u255E', boxVr: '\u255F', boxVR: '\u2560',
    boxvL: '\u2561', boxVl: '\u2562', boxVL: '\u2563', boxHd: '\u2564',
    boxhD: '\u2565', boxHD: '\u2566', boxHu: '\u2567', boxhU: '\u2568',
    boxHU: '\u2569', boxvH: '\u256A', boxVh: '\u256B', boxVH: '\u256C',

    // ── Block Elements / Geometric Shapes ─────────────────────────────────
    uhblk: '\u2580', lhblk: '\u2584', block: '\u2588', blk14: '\u2591',
    blk12: '\u2592', blk34: '\u2593', squ: '\u25A1', squf: '\u25AA',
    EmptyVerySmallSquare: '\u25AB', rect: '\u25AD', marker: '\u25AE',
    fltns: '\u25B1', xutri: '\u25B3', utrif: '\u25B4', utri: '\u25B5',
    rtrif: '\u25B8', rtri: '\u25B9', xdtri: '\u25BD', dtrif: '\u25BE',
    dtri: '\u25BF', ltrif: '\u25C2', ltri: '\u25C3', loz: '\u25CA',
    cir: '\u25CB', tridot: '\u25EC', xcirc: '\u25EF', ultri: '\u25F8',
    urtri: '\u25F9', lltri: '\u25FA', EmptySmallSquare: '\u25FB',
    FilledSmallSquare: '\u25FC',

    // ── Miscellaneous Symbols (U+2600–U+26FF) ─────────────────────────────
    starf: '\u2605', star: '\u2606', phone: '\u260E', female: '\u2640',
    male: '\u2642', spades: '\u2660', clubs: '\u2663', hearts: '\u2665',
    diams: '\u2666', sung: '\u266A', flat: '\u266D', natur: '\u266E',
    sharp: '\u266F',

    // ── Dingbats ──────────────────────────────────────────────────────────
    check: '\u2713', cross: '\u2717', malt: '\u2720', sext: '\u2736',
    VerticalSeparator: '\u2758', lbbrk: '\u2772', rbbrk: '\u2773',

    // ── Supplemental Arrows / Math Symbols ────────────────────────────────
    lobrk: '\u27E6', robrk: '\u27E7', lang: '\u27E8', rang: '\u27E9',

    // ── Supplemental Mathematical Operators (U+2A00–U+2AFF) ───────────────
    xodot: '\u2A00', xoplus: '\u2A01', xotime: '\u2A02', xuplus: '\u2A04',
    xsqcup: '\u2A06', qint: '\u2A0C', fpartint: '\u2A0D', cirfnint: '\u2A10',
    awint: '\u2A11', rppolint: '\u2A12', scpolint: '\u2A13', npolint: '\u2A14',
    pointint: '\u2A15', quatint: '\u2A16', intlarhk: '\u2A17', pluscir: '\u2A22',
    plusacir: '\u2A23', simplus: '\u2A24', plusdu: '\u2A25', plussim: '\u2A26',
    plustwo: '\u2A27', mcomma: '\u2A29', minusdu: '\u2A2A', loplus: '\u2A2D',
    roplus: '\u2A2E', Cross: '\u2A2F', timesd: '\u2A30', timesbar: '\u2A31',
    smashp: '\u2A33', lotimes: '\u2A34', rotimes: '\u2A35', otimesas: '\u2A36',
    Otimes: '\u2A37', odiv: '\u2A38', triplus: '\u2A39', triminus: '\u2A3A',
    tritime: '\u2A3B', iprod: '\u2A3C', amalg: '\u2A3F', capdot: '\u2A40',
    ncup: '\u2A42', ncap: '\u2A43', capand: '\u2A44', cupor: '\u2A45',
    cupcap: '\u2A46', capcup: '\u2A47', cupbrcap: '\u2A48', capbrcup: '\u2A49',
    cupcup: '\u2A4A', capcap: '\u2A4B', ccups: '\u2A4C', ccaps: '\u2A4D',
    ccupssm: '\u2A50', And: '\u2A53', Or: '\u2A54', andand: '\u2A55',
    oror: '\u2A56', orslope: '\u2A57', andslope: '\u2A58', andv: '\u2A5A',
    orv: '\u2A5B', andd: '\u2A5C', ord: '\u2A5D', wedbar: '\u2A5F',
    sdote: '\u2A66', simdot: '\u2A6A', congdot: '\u2A6D', easter: '\u2A6E',
    apacir: '\u2A6F', apE: '\u2A70', eplus: '\u2A71', pluse: '\u2A72',
    Esim: '\u2A73', Colone: '\u2A74', Equal: '\u2A75', eDDot: '\u2A77',
    equivDD: '\u2A78', ltcir: '\u2A79', gtcir: '\u2A7A', ltquest: '\u2A7B',
    gtquest: '\u2A7C', les: '\u2A7D', ges: '\u2A7E', lesdot: '\u2A7F',
    gesdot: '\u2A80', lesdoto: '\u2A81', gesdoto: '\u2A82', lesdotor: '\u2A83',
    gesdotol: '\u2A84', lap: '\u2A85', gap: '\u2A86', lne: '\u2A87',
    gne: '\u2A88', lnap: '\u2A89', gnap: '\u2A8A', lEg: '\u2A8B',
    gEl: '\u2A8C', lsime: '\u2A8D', gsime: '\u2A8E', lsimg: '\u2A8F',
    gsiml: '\u2A90', lgE: '\u2A91', glE: '\u2A92', lesges: '\u2A93',
    gesles: '\u2A94', els: '\u2A95', egs: '\u2A96', elsdot: '\u2A97',
    egsdot: '\u2A98', el: '\u2A99', eg: '\u2A9A', siml: '\u2A9D',
    simg: '\u2A9E', simlE: '\u2A9F', simgE: '\u2AA0', LessLess: '\u2AA1',
    GreaterGreater: '\u2AA2', glj: '\u2AA4', gla: '\u2AA5', ltcc: '\u2AA6',
    gtcc: '\u2AA7', lescc: '\u2AA8', gescc: '\u2AA9', smt: '\u2AAA',
    lat: '\u2AAB', smte: '\u2AAC', late: '\u2AAD', bumpE: '\u2AAE',
    pre: '\u2AAF', sce: '\u2AB0', prE: '\u2AB3', scE: '\u2AB4',
    prnE: '\u2AB5', scnE: '\u2AB6', prap: '\u2AB7', scap: '\u2AB8',
    prnap: '\u2AB9', scnap: '\u2ABA', Pr: '\u2ABB', Sc: '\u2ABC',
    subdot: '\u2ABD', supdot: '\u2ABE', subplus: '\u2ABF', supplus: '\u2AC0',
    submult: '\u2AC1', supmult: '\u2AC2', subedot: '\u2AC3', supedot: '\u2AC4',
    subE: '\u2AC5', supE: '\u2AC6', subsim: '\u2AC7', supsim: '\u2AC8',
    subnE: '\u2ACB', supnE: '\u2ACC', csub: '\u2ACF', csup: '\u2AD0',
    csube: '\u2AD1', csupe: '\u2AD2', subsup: '\u2AD3', supsub: '\u2AD4',
    subsub: '\u2AD5', supsup: '\u2AD6', suphsub: '\u2AD7', supdsub: '\u2AD8',
    forkv: '\u2AD9', topfork: '\u2ADA', mlcp: '\u2ADB', Dashv: '\u2AE4',
    Vdashl: '\u2AE6', Barv: '\u2AE7', vBar: '\u2AE8', vBarv: '\u2AE9',
    Vbar: '\u2AEB', Not: '\u2AEC', bNot: '\u2AED', rnmid: '\u2AEE',
    cirmid: '\u2AEF', midcir: '\u2AF0', topcir: '\u2AF1', nhpar: '\u2AF2',
    parsim: '\u2AF3', parsl: '\u2AFD',
};

/** Character-encoding map for safe embedding in a <title> text node. */
const TITLE_ENCODE_MAP = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' };

/**
 * Decodes decimal (&#N;), hex (&#xN;), and named HTML5 character references
 * in a single pass. Unrecognised named references are left as-is.
 *
 * Single-pass is required for correctness: sequential replaces would
 * double-decode constructs like &#38;lt; (→ &lt; → <) when the correct
 * plain-text result is &lt; (only the numeric ref is decoded; the resulting
 * ampersand is not re-interpreted as a new entity opener).
 * @param {string} str
 * @returns {string}
 */
function decodeEntities(str) {
    return str.replace(/&(?:#(\d+)|#[xX]([0-9a-fA-F]+)|(\w+));/g, (m, dec, hex, name) => {
        if (dec !== undefined) {
            const cp = Number(dec);
            // Reject surrogates (U+D800–U+DFFF): String.fromCodePoint throws RangeError for them.
            return (cp >= 0 && cp <= MAX_CODE_POINT && (cp < SURROGATE_MIN || cp > SURROGATE_MAX)) ? String.fromCodePoint(cp) : m;
        }
        if (hex !== undefined) {
            const cp = parseInt(hex, 16);
            return (cp >= 0 && cp <= MAX_CODE_POINT && (cp < SURROGATE_MIN || cp > SURROGATE_MAX)) ? String.fromCodePoint(cp) : m;
        }
        return Object.hasOwn(HTML_ENTITIES, name) ? HTML_ENTITIES[name] : m;
    });
}

// ── HTML → plain-text renderer ────────────────────────────────────────────────

/** HTML void elements — self-closing by spec; never produce close tokens. */
const VOID_TAGS = new Set([
    'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
    'link', 'meta', 'param', 'source', 'track', 'wbr',
]);

/**
 * Tokenise an HTML string into an array of token objects.
 * Each token is one of:
 *   { type: 'text',  raw: string }
 *   { type: 'open',  tag: string, attrs: string, selfClose: boolean }
 *   { type: 'close', tag: string }
 *   { type: 'comment' }
 *
 * Attribute quoting and embedded `>` inside quoted values are handled
 * by the same attribute grammar used in toPlainText's old tag regex.
 */
function tokenize(html) {
    const TOKEN_RE = /<!--[\s\S]*?-->|<(\/?)(\w[\w-]*)(\s(?:"[^"]*"|'[^']*'|[^>])*)?(\/)?>|([^<]+)/g;
    const tokens = [];
    let m;
    while ((m = TOKEN_RE.exec(html)) !== null) {
        if (m[0].startsWith('<!--')) {
            tokens.push({ type: 'comment' });
        } else if (m[2]) {
            const tag = m[2].toLowerCase();
            const isClose = m[1] === '/';
            const attrs = m[3] || '';
            const selfClose = m[4] === '/' || VOID_TAGS.has(tag);
            tokens.push(isClose
                ? { type: 'close', tag }
                : { type: 'open', tag, attrs, selfClose });
        } else if (m[5] !== undefined) {
            tokens.push({ type: 'text', raw: m[5] });
        }
    }
    return tokens;
}

/** Block-level elements that force blank-line separation in plain text. */
const BLOCK_TAGS = new Set([
    'address', 'article', 'aside', 'blockquote', 'caption', 'dd', 'details',
    'dialog', 'div', 'dl', 'dt', 'fieldset', 'figcaption', 'figure', 'footer',
    'form', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'header', 'hgroup', 'hr',
    'legend', 'li', 'main', 'nav', 'ol', 'p', 'pre', 'section', 'summary',
    'table', 'tbody', 'td', 'tfoot', 'th', 'thead', 'tr', 'ul',
]);

/** Tags whose text content should be suppressed entirely (navigation noise). */
const SKIP_TAGS = new Set(['script', 'style', 'noscript', 'head']);

/** Number of distinct list-style types before the cycle repeats. */
const OL_CYCLE = 3;

/** Unordered-list bullet characters by nesting depth (cycles at OL_CYCLE). */
const UL_BULLETS = ['*', '-', '+'];

/** Alphabetic markers for depth-1 ordered lists (a–z, then wraps). */
const OL_ALPHA = 'abcdefghijklmnopqrstuvwxyz';

/**
 * Roman-numeral lookup for depth-2 ordered lists (counters 1–26, then wraps).
 * Covers the full Latin alphabet in roman form; wraps beyond that.
 */
const OL_ROMAN = [
    'i','ii','iii','iv','v','vi','vii','viii','ix','x',
    'xi','xii','xiii','xiv','xv','xvi','xvii','xviii','xix','xx',
    'xxi','xxii','xxiii','xxiv','xxv','xxvi',
];

/**
 * Returns the list-item marker for an ordered list.
 * Depth 0 → decimal (1.), depth 1 → alpha (a.), depth 2 → roman (i.).
 * All three cycle: OL_CYCLE controls how many types exist.
 */
function olMarker(counter, depth) {
    const d = depth % OL_CYCLE;
    if (d === 0) { return `${counter}.`; }
    if (d === 1) { return `${OL_ALPHA[(counter - 1) % OL_ALPHA.length]}.`; }
    return `${OL_ROMAN[(counter - 1) % OL_ROMAN.length]}.`;
}

/** Pre-compiled attribute regexes keyed by attribute name (href, alt, src). */
const ATTR_RE_CACHE = new Map();

/**
 * Extract the value of a named attribute from an attribute string.
 * Handles double-quoted, single-quoted, and unquoted forms.
 * @param {string} attrs Raw attribute string from the opening tag.
 * @param {string} name  Attribute name (case-insensitive).
 * @returns {string|null}
 */
function getAttr(attrs, name) {
    let re = ATTR_RE_CACHE.get(name);
    if (!re) {
        re = new RegExp(`\\b${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)'|([^\\s>]*))`, 'i');
        ATTR_RE_CACHE.set(name, re);
    }
    const m = re.exec(attrs);
    return m ? (m[1] ?? m[2] ?? m[3] ?? '') : null;
}

/**
 * Converts an HTML fragment to plain text with structural fidelity:
 *   - Links:      text [url]  (url omitted for #fragments or when text === url)
 *   - Headings:   underlined with = / - or prefixed with ## / ###
 *   - Lists:      bulleted (* / - / +) or numbered (1. / a. / i.) with nesting
 *   - Blockquote: each line prefixed with ">  "
 *   - Pre/code:   whitespace preserved; inline code in `backticks`
 *   - Paragraphs: separated by a blank line
 *   - br / hr:    newline / horizontal rule
 *   - nbsp etc.:  decoded and normalised to regular spaces
 *
 * @param {string} fragment
 * @returns {string}
 */
export function toPlainText(fragment) {
    const tokens = tokenize(fragment);

    // ── Renderer state ──────────────────────────────────────────────────────
    const parts = [];          // output segments
    let skipDepth = 0;         // >0 means we are inside a SKIP_TAG subtree
    let preDepth = 0;          // >0 means inside <pre>
    let bqDepth = 0;           // current blockquote nesting level
    const bqStack = [];        // parts[] index at the time each <blockquote> opened
    let inlineCodeDepth = 0;   // <code> not inside <pre>

    // List stacks — parallel arrays of { tag, counter, depth }
    const listStack = [];      // { tag:'ul'|'ol', counter:number }
    let inLi = false;          // currently building an <li> content block
    const liParts = [];        // accumulates inline content of current <li>

    // Heading state
    let headingTag = null;     // 'h1'…'h6' while inside a heading
    const headingParts = [];

    // Link state
    let linkHref = null;
    let linkText = '';

    // ── Helpers ─────────────────────────────────────────────────────────────

    function push(str) {
        if (skipDepth > 0) { return; }
        if (headingTag) {
            if (linkHref !== null) { linkText += str; return; }
            headingParts.push(str);
            return;
        }
        if (inLi) {
            if (linkHref !== null) { linkText += str; return; }
            liParts.push(str);
            return;
        }
        if (linkHref !== null) { linkText += str; return; }
        parts.push(str);
    }

    function ensureBlankLine() {
        // Scan backwards over any empty strings to find the real last part.
        let i = parts.length - 1;
        while (i >= 0 && !parts[i]) { i--; }
        if (i < 0) { return; }
        const last = parts[i];
        if (!last.endsWith('\n\n')) {
            parts.push(last.endsWith('\n') ? '\n' : '\n\n');
        }
    }

    function flushHeading() {
        if (!headingTag) { return; }
        const text = decodeEntities(headingParts.join('').trim());
        headingParts.length = 0;
        ensureBlankLine();
        if (headingTag === 'h1') {
            parts.push(`${text}\n${'='.repeat(text.length)}\n\n`);
        } else if (headingTag === 'h2') {
            parts.push(`${text}\n${'-'.repeat(text.length)}\n\n`);
        } else {
            const level = Number(headingTag[1]);
            parts.push(`${'#'.repeat(level)} ${text}\n\n`);
        }
        headingTag = null;
    }

    function flushLi() {
        if (!inLi) { return; }
        const raw = liParts.join('');
        liParts.length = 0;
        inLi = false;
        const level = listStack.length - 1;
        const listInfo = listStack[level];
        const indent = '  '.repeat(level);
        let marker;
        if (listInfo.tag === 'ul') {
            marker = UL_BULLETS[level % UL_BULLETS.length];
        } else {
            marker = olMarker(listInfo.counter, level);
            listInfo.counter++;
        }
        // Trim and normalise interior whitespace of the li text, preserving
        // inner newlines from nested blocks.
        const text = raw.replace(/[ \t]+/g, ' ').trim();
        const contIndent = `${indent}  `;
        const indented = text.split('\n').map((l, i) => i === 0 ? l : `${contIndent}${l}`).join('\n');
        parts.push(`${indent}${marker} ${indented}\n`);
    }

    // ── Token walk ──────────────────────────────────────────────────────────

    for (const tok of tokens) {
        if (tok.type === 'comment') { continue; }

        if (tok.type === 'text') {
            if (skipDepth > 0) { continue; }
            const decoded = decodeEntities(tok.raw);
            // Normalise whitespace outside <pre>; push() handles all context routing.
            push(preDepth > 0 ? decoded : decoded.replace(/[\r\n\t ]+/g, ' '));
            continue;
        }

        const { tag } = tok;

        if (tok.type === 'open') {
            // ── skip subtrees ──────────────────────────────────────────────
            if (SKIP_TAGS.has(tag)) { skipDepth++; continue; }
            if (skipDepth > 0) { continue; }

            switch (tag) {
            // ── structural blocks ──────────────────────────────────────
            case 'p':
            case 'div':
            case 'article':
            case 'section':
            case 'main':
            case 'header':
            case 'footer':
            case 'nav':
            case 'aside':
            case 'figure':
            case 'figcaption':
            case 'address':
                ensureBlankLine();
                break;

                // ── headings ───────────────────────────────────────────────
            case 'h1': case 'h2': case 'h3':
            case 'h4': case 'h5': case 'h6':
                flushLi();
                headingTag = tag;
                break;

                // ── lists ──────────────────────────────────────────────────
            case 'ul':
            case 'ol':
                flushLi();
                listStack.push({ tag, counter: 1 });
                break;

            case 'li':
                flushLi();
                inLi = true;
                break;

            case 'dt':
                ensureBlankLine();
                break;

            case 'dd':
                push('\n    ');
                break;

                // ── blockquote ─────────────────────────────────────────────
            case 'blockquote':
                bqDepth++;
                ensureBlankLine();
                bqStack.push(parts.length);
                break;

                // ── pre / code ─────────────────────────────────────────────
            case 'pre':
                preDepth++;
                ensureBlankLine();
                break;

            case 'code':
                if (preDepth === 0) {
                    inlineCodeDepth++;
                    push('`');
                }
                break;

                // ── inline formatting (no visible marker in .txt) ──────────
            case 'b': case 'strong':
            case 'i': case 'em':
            case 'u': case 's': case 'del':
            case 'mark': case 'small': case 'sup': case 'sub':
            case 'abbr': case 'cite': case 'dfn': case 'q':
                break;

                // ── links ──────────────────────────────────────────────────
            case 'a': {
                const href = getAttr(tok.attrs, 'href');
                if (href && !href.startsWith('#')) {
                    linkHref = href;
                    linkText = '';
                }
                break;
            }

            // ── void / replaced elements ───────────────────────────────
            case 'br':
                push('\n');
                break;

            case 'hr':
                ensureBlankLine();
                parts.push('────────────────────────────────────────\n\n');
                break;

            case 'img': {
                const alt = getAttr(tok.attrs, 'alt');
                if (alt && alt.trim()) { push(`[${alt.trim()}]`); }
                break;
            }

            // ── table cells — separate with a tab-like gap ────────────
            case 'th':
            case 'td':
                push('  ');
                break;

            case 'tr':
                push('\n');
                break;

                // ── description list terms ─────────────────────────────────
            case 'dl':
                ensureBlankLine();
                break;

            default:
                if (BLOCK_TAGS.has(tag)) { ensureBlankLine(); }
                break;
            }

            continue;
        }

        // ── close tokens ──────────────────────────────────────────────────
        if (tok.type === 'close') {
            if (SKIP_TAGS.has(tag)) { if (skipDepth > 0) { skipDepth--; } continue; }
            if (skipDepth > 0) { continue; }

            switch (tag) {
            // ── headings ───────────────────────────────────────────────
            case 'h1': case 'h2': case 'h3':
            case 'h4': case 'h5': case 'h6':
                flushHeading();
                break;

                // ── lists ──────────────────────────────────────────────────
            case 'li':
                flushLi();
                break;

            case 'ul':
            case 'ol':
                flushLi();
                listStack.pop();
                if (listStack.length === 0) { ensureBlankLine(); }
                break;

                // ── blocks ─────────────────────────────────────────────────
            case 'p':
            case 'div':
            case 'article':
            case 'section':
            case 'main':
            case 'header':
            case 'footer':
            case 'nav':
            case 'aside':
            case 'figure':
            case 'figcaption':
            case 'address':
            case 'dl':
                ensureBlankLine();
                break;

                // ── blockquote ─────────────────────────────────────────────
            case 'blockquote': {
                ensureBlankLine();
                if (bqDepth > 0 && bqStack.length > 0) {
                    const depth = bqDepth;
                    bqDepth--;
                    const startIdx = bqStack.pop();
                    const inner = parts.splice(startIdx).join('');
                    const prefix = '> '.repeat(depth);
                    parts.push(inner.split('\n').map(l => (l ? `${prefix}${l}` : l)).join('\n'));
                }
                break;
            }

            // ── pre / code ─────────────────────────────────────────────
            case 'pre':
                if (preDepth > 0) { preDepth--; }
                ensureBlankLine();
                break;

            case 'code':
                if (preDepth === 0 && inlineCodeDepth > 0) {
                    inlineCodeDepth--;
                    push('`');
                }
                break;

                // ── links ──────────────────────────────────────────────────
            case 'a': {
                if (linkHref !== null) {
                    const text = linkText.replace(/[\r\n\t ]+/g, ' ').trim();
                    const href = linkHref;
                    linkHref = null;
                    linkText = '';
                    // Avoid duplicating the URL when the link text IS the URL.
                    if (text && text === href) {
                        push(text);
                    } else if (text) {
                        push(`${text} [${href}]`);
                    } else {
                        push(`[${href}]`);
                    }
                }
                break;
            }

            // ── table rows ─────────────────────────────────────────────
            case 'tr':
            case 'tbody':
            case 'thead':
            case 'tfoot':
                push('\n');
                break;

            case 'table':
                ensureBlankLine();
                break;

            default:
                if (BLOCK_TAGS.has(tag)) { ensureBlankLine(); }
                break;
            }
        }
    }

    // Flush any unclosed <li> or <heading> at end-of-fragment.
    flushLi();
    flushHeading();

    let result = parts.join('');

    // Normalise: collapse 3+ newlines → 2; strip only boundary newlines (not
    // spaces — leading spaces in <pre> blocks must survive).
    result = result.replace(/\n{3,}/g, '\n\n').replace(/^\n+/, '').replace(/\n+$/, '');

    // Convert non-breaking spaces to regular spaces in the final output.
    result = result.replace(/\u00A0/g, ' ');

    return result;
}

/**
 * Encodes HTML special characters for safe embedding in a <title> text node.
 * Decodes entity references first to prevent double-encoding.
 * @param {string} str
 * @returns {string}
 */
function encodeHtmlTitle(str) {
    return decodeEntities(str).replace(/[&<>"]/g, c => TITLE_ENCODE_MAP[c]);
}

/**
 * Derives a meaningful <title> for the HTML wrapper.
 * Priority: (1) <title> in the extracted fragment, (2) hostname for URLs or
 * filename stem for local paths, (3) generic fallback.
 * @param {string} content The extracted HTML content.
 * @param {string} source The original input path or URL.
 * @returns {string}
 */
function deriveTitle(content, source) {
    const titleMatch = /<title[^>]*>([^<]+)<\/title>/i.exec(content);
    if (titleMatch) {
        const t = titleMatch[1].trim();
        if (t) { return t; }
    }

    // '-' is the stdin sentinel value — it has no meaningful title.
    if (source && source !== '-') {
        try {
            return new URL(source).hostname;
        } catch {
            const stem = basename(source).replace(/\.[^.]+$/, '');
            if (stem) { return stem; }
        }
    }

    return 'HTMLCut Extraction';
}

/**
 * Builds a timestamped, uniquely-suffixed base path shared by both output files
 * of a single extraction. Append the extension ('.html' or '.txt') to form the
 * final path — both files must use the same base so the pair is visually obvious.
 * @param {string} basePath The base path requested by the user.
 * @returns {string} The output base path (without extension).
 */
export function getOutputBase(basePath) {
    const d = new Date();
    const pad = n => String(n).padStart(2, '0');
    const dateStr = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}-${pad(d.getHours())}-${pad(d.getMinutes())}-${pad(d.getSeconds())}`;
    const hash = randomBytes(2).toString('hex');
    return `${basePath}-htmlcut-${dateStr}-${hash}`;
}

/**
 * Writes content to the filesystem using fs/promises.
 * @param {string} content The extracted string content.
 * @param {string} outputPath The destination file path.
 * @param {string} [source] The original input path or URL, used to derive a meaningful title.
 * @returns {Promise<void>}
 */
export async function writeOutput(content, outputPath, source = '') {
    let finalContent = content;

    if (outputPath.endsWith('.html')) {
        if (!/^\s*(?:<!DOCTYPE\b|<html\b)/i.test(finalContent)) {
            const title = encodeHtmlTitle(deriveTitle(finalContent, source));
            finalContent = `<!DOCTYPE html>\n<html lang="en">\n<head>\n    <meta charset="utf-8">\n    <meta name="viewport" content="width=device-width, initial-scale=1">\n    <title>${title}</title>\n</head>\n<body>\n${finalContent}\n</body>\n</html>`;
        }
    }

    await writeFile(outputPath, finalContent, 'utf8');
}
