import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const linksPath = path.join(here, 'links.json');

const groupTags = {
  'Programming Learning - Programmer Forum': ['forum', 'programming'],
  'Programming Learning - Programming Language Tutorial': ['tutorial', 'programming'],
  'Programming Learning - Website Deployment and Maintenance': ['deployment', 'tools'],
  'Academic Research - Microbiology Field': ['research', 'microbiology'],
  'Academic Research - Database Website': ['research', 'database'],
  'Academic Research - Tool Tutorial(EN)': ['tutorial', 'tools'],
  'Academic Research - Tool Tutorial(CN)': ['tutorial', 'Chinese'],
  'Academic Research - Common Websites': ['common', 'tools'],
  'Academic Research - Literature search': ['literature', 'research'],
  'Academic Research - Journal classification': ['journal', 'research'],
  'Personal Section - Helloxiaolaodi': ['personal', 'GitHub'],
  'Online Translation - Translation Website': ['translation', 'tools'],
  'Tools - Cloud Storage Miscellaneous': ['tools', 'cloud storage'],
  'Tools - Cycling': ['cycling', 'sports'],
  'Tools - References': ['literature', 'tools'],
  'Tools - Online Conversion': ['conversion', 'tools'],
  'Tools - Ladder Proxy': ['proxy', 'tools'],
  'Tools - AI model': ['AI', 'tools'],
  'NCU - Daily affairs': ['campus', 'NCU'],
};

const keywordTags = [
  [/python|pandas|jupyter|anaconda|miniconda/i, 'Python'],
  [/\br\b|bioconductor|cran|rstudio|r4ds/i, 'R'],
  [/nodejs/i, 'Node'],
  [/git|github|gist/i, 'Git'],
  [/qiime/i, 'QIIME2'],
  [/biocyc|greengenes|homd|gtdb|qiita|ena|ddbj|cncb|metacyc|rrndb|jbrowse|blast|ncbi/i, 'bioinformatics'],
  [/blast|ncbi|scopus|elsevier|pubmed|medcite/i, 'database'],
  [/google|scholar|bing|baidu|gfsoso|stork|sc\.panda|so\.673|lanfanshu|sci-hub|gupiao|ablesci|9312/i, 'search'],
  [/latex|overleaf/i, 'LaTeX'],
  [/ref-extractor|anystyle|doi2bib|citationstyles|zotero/i, 'citation'],
  [/grammarly|quillbot|paperbert/i, 'writing'],
  [/translate|fanyi|cnki/i, 'translation'],
  [/clash|guatizi|sakura|mojie|proxy|vpn/i, 'proxy'],
  [/deepseek|gemini|opencode|hugging/i, 'AI'],
  [/spotify|music/i, 'music'],
  [/tour|uci|cycling|procycling|vuelta|giro|paris|letour/i, 'racing'],
  [/pdf|convert|smallpdf|alltoall|online-convert/i, 'PDF'],
  [/ncu/i, 'NCU'],
  [/mail/i, 'email'],
  [/paypal/i, 'payment'],
  [/anki/i, 'memory'],
  [/iconfont/i, 'design'],
  [/base64|toolhelper/i, 'encoding'],
];

const vpnRequired = new Set([
  'https://linux.do',
  'https://credit.linux.do',
  'https://cdk.linux.do/dashboard',
  'https://wesmckinney.com/book',
  'https://r4ds.hadley.nz',
  'https://huggingface.co',
  'https://gist.github.com',
  'https://github.com/biobakery/biobakery/wiki/MaAsLin3',
  'https://github.com/picrust/picrust2/wiki/Full-pipeline-script',
  'https://biocyc.org/webinar.shtml',
  'https://rrndb.umms.med.umich.edu',
  'https://metacyc.org',
  'https://github.com',
  'https://github.com/Helloxiaolaodi',
  'https://www.google.com',
  'https://scholar.google.com',
  'https://seeing-theory.brown.edu/#firstPage',
  'https://translate.google.com',
  'https://archive.org/details/sonofheavenabiog0000fitz/mode/2up',
  'https://webmail.linux.do/sso/login',
  'https://open.spotify.com',
  'https://gemini.google.com',
]);

const links = JSON.parse(await readFile(linksPath, 'utf8'));

for (const group of links.icons || []) {
  const baseTags = groupTags[group.title] || ['navigation'];
  for (const item of group.children || []) {
    const tags = new Set(baseTags);
    const searchText = `${item.title} ${item.url} ${item.description || ''}`;
    for (const [pattern, tag] of keywordTags) {
      if (pattern.test(searchText)) tags.add(tag);
    }
    item.tags = [...tags];
    item.isVpnRequired = vpnRequired.has(item.url.replace(/\?.*$/, '').replace(/\/+$/, ''));
    item.clickCount = Number.isInteger(item.clickCount) ? item.clickCount : 0;
  }
}

const json = JSON.stringify(links, null, 4) + '\n';
links.md5 = createHash('md5').update(json).digest('hex');
await writeFile(linksPath, JSON.stringify(links, null, 4) + '\n', 'utf8');

const allItems = links.icons.flatMap((group) => group.children || []);
const tagSet = new Set(allItems.flatMap((item) => item.tags || []));
console.log(`items=${allItems.length}`);
console.log(`vpn_required=${allItems.filter((item) => item.isVpnRequired).length}`);
console.log(`tags=${[...tagSet].sort((a, b) => a.localeCompare(b, 'en')).join(', ')}`);
console.log(`md5=${links.md5}`);
