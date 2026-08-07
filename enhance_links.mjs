import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const linksPath = path.join(here, 'links.json');

const groupTags = {
  'Programming Learning - Programmer Forum': ['论坛', '编程'],
  'Programming Learning - Programming Language Tutorial': ['教程', '编程'],
  'Programming Learning - Website Deployment and Maintenance': ['部署', '工具'],
  'Academic Research - Microbiology Field': ['科研', '微生物'],
  'Academic Research - Database Website': ['科研', '数据库'],
  'Academic Research - Tool Tutorial(EN)': ['教程', '工具'],
  'Academic Research - Tool Tutorial(CN)': ['教程', '中文'],
  'Academic Research - Common Websites': ['常用', '工具'],
  'Academic Research - Literature search': ['文献', '科研'],
  'Academic Research - Journal classification': ['期刊', '科研'],
  'Personal Section - Helloxiaolaodi': ['个人', 'GitHub'],
  'Online Translation - Translation Website': ['翻译', '工具'],
  'Tools - Cloud Storage Miscellaneous': ['工具', '云存储'],
  'Tools - Cycling': ['骑行', '运动'],
  'Tools - References': ['文献', '工具'],
  'Tools - Online Conversion': ['转换', '工具'],
  'Tools - Ladder Proxy': ['代理', '工具'],
  'Tools - AI model': ['AI', '工具'],
  'NCU - Daily affairs': ['校园', 'NCU'],
};

const keywordTags = [
  [/python|pandas|jupyter|anaconda|miniconda/i, 'Python'],
  [/\br\b|bioconductor|cran|rstudio|r4ds/i, 'R'],
  [/nodejs/i, 'Node'],
  [/git|github|gist/i, 'Git'],
  [/qiime/i, 'QIIME2'],
  [/biocyc|greengenes|homd|gtdb|qiita|ena|ddbj|cncb|metacyc|rrndb|jbrowse|blast|ncbi/i, '生信'],
  [/blast|ncbi|scopus|elsevier|pubmed|medcite/i, '数据库'],
  [/google|scholar|bing|baidu|gfsoso|stork|sc\.panda|so\.673|lanfanshu|sci-hub|gupiao|ablesci|9312/i, '搜索'],
  [/latex|overleaf/i, 'LaTeX'],
  [/ref-extractor|anystyle|doi2bib|citationstyles|zotero/i, '引文'],
  [/grammarly|quillbot|paperbert/i, '写作'],
  [/translate|fanyi|cnki/i, '翻译'],
  [/clash|guatizi|sakura|mojie|proxy|vpn/i, '代理'],
  [/deepseek|gemini|opencode|hugging/i, 'AI'],
  [/spotify|music/i, '音乐'],
  [/tour|uci|cycling|procycling|vuelta|giro|paris|letour/i, '赛事'],
  [/pdf|convert|smallpdf|alltoall|online-convert/i, 'PDF'],
  [/ncu/i, 'NCU'],
  [/mail/i, '邮件'],
  [/paypal/i, '支付'],
  [/anki/i, '记忆'],
  [/iconfont/i, '设计'],
  [/base64|toolhelper/i, '编码'],
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
  const baseTags = groupTags[group.title] || ['导航'];
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
console.log(`tags=${[...tagSet].sort((a, b) => a.localeCompare(b, 'zh-CN')).join(', ')}`);
console.log(`md5=${links.md5}`);
