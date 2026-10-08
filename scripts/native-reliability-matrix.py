#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Current contract controls against the supplied real native executable, without acquisition."""
import argparse, hashlib, json, subprocess, tempfile, tomllib
from pathlib import Path


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();package_version=tomllib.loads((Path(__file__).resolve().parents[1]/"Cargo.toml").read_text())["workspace"]["package"]["version"];binary=args.binary.resolve();before=hashlib.sha256(binary.read_bytes()).hexdigest();rows=[]
    def run(label,arguments,data=None,expected=None,raw=None,failure=None):
        result=subprocess.run([str(binary),*arguments],input=data,capture_output=True,timeout=20)
        passed=result.returncode==(failure or 0)
        if failure:passed &= result.stdout==b'' and json.loads(result.stderr)['version']==wire_version
        else:
            passed &= result.stderr==b''
            if raw is not None:passed &= result.stdout==raw
            if expected is not None:passed &= json.loads(result.stdout)==expected
        rows.append(dict(label=label,arguments=arguments,exit_code=result.returncode,passed=passed,
                         stdout=result.stdout.decode('utf-8'),stderr=result.stderr.decode('utf-8')))
        return result
    wire_version=json.loads(subprocess.run([str(binary),'schema','htmlcut.extraction.plan'],check=True,capture_output=True).stdout)['properties']['version']['const']
    run('binary-version',['--version'],raw=f'htmlcut {package_version}\n'.encode())
    with tempfile.TemporaryDirectory(prefix='htmlcut-native-contract-') as directory:
        root=Path(directory)
        for label,source,css,projection,expected in [
            ('literal-hidden','<p>A<span hidden>B</span><template>T</template></p>','p','literal',['ABT']),
            ('reading-hidden','<p>A<span hidden>B</span><template>T</template><script>S</script></p>','p','markdown',['AB']),
            ('reference-label','<p>Before <a class="reference internal" href="next">IMPORTANT</a> after.</p>','p','markdown',['Before [IMPORTANT](<next>) after.']),
            ('definition-blocks','<dl><dt>Name</dt><dd>Alice</dd></dl>','dl','markdown',['Name\n\nAlice']),
            ('source-ordinals','<ol start="3"><li>A</li><li value="8">B</li></ol>','ol','markdown',['- 3\\. A\n- 8\\. B']),
            ('table-cells','<table><tr><th>A</th><td>B</td></tr></table>','table','markdown',['-\n  - <strong>A</strong>\n  - B']),
            ('pre-fragment','<pre>OUTSIDE<code id="x"> x\n</code>TAIL</pre>','#x','markdown',['```\n x\n\n```']),
            ('outer-html','<P data-x="">V</P>','p','outer-html',['<p data-x="">V</p>'])]:
            run(label,['extract','--stdin','--select',css,'--read',projection],source.encode(),expected=expected)
        for label,source,css,reading,expected in [
            ('inline-code','<p>x<code>a`b</code>y</p>','p','markdown',['x``a`b``y']),
            ('punctuation-emphasis','<p>x<em>!</em>y<strong>z</strong></p>','p','markdown',['x<em>\\!</em>y<strong>z</strong>']),
            ('literal-character-reference','<p>&amp;copy; &amp;#x41;</p>','p','markdown',['\\&copy; \\&#x41;']),
            ('explicit-language','<pre><code class="language-rust">x</code></pre>','pre','markdown',['```rust\nx\n```']),
            ('unicode-normalization','<p>\u00a0a\u2003b\u202f</p>','p','text',['a b']),
            ('zero-width-literal','<p>\u200ba\u200b</p>','p','text',['\u200ba\u200b'])]:
            run(label,['extract','--stdin','--select',css,'--read',reading],source.encode(),expected=expected)
        group=['extract','--stdin','--select','tr.entry','--field','title',':scope td','literal',
               '--field','score',':scope + tr .score','text','--following-siblings','1']
        run('explicit-row-group',group,b'<table><tr class=entry><td>T</td></tr><!-- gap --><tr><td class=score> 10 </td></tr></table>',expected=[dict(title='T',score='10')])
        run('optional-inline-field',['extract','--stdin','--select','article','--all','--optional-field','score','.score','text'],b'<article><b class=score>10</b></article><article></article>',expected=[dict(score='10'),dict(score=None)])
        run('inline-field-exclusion',['extract','--stdin','--select','tr','--field','country','td','text','--field-exclude','country','sup.reference'],b'<table><tr><td>China<sup class=reference>[1]</sup></td></tr></table>',expected=[dict(country='China')])
        run('scalar-exclusion',['extract','--stdin','--select','p','--read','markdown','--exclude','sup.reference'],b'<p>China<sup class=reference>[1]</sup></p>',expected=['China'])
        header_args=['extract','--stdin','--select','tr:has(td)','--field','location','td:nth-child(1)','text','--expect-text','th:nth-child(1)','Location']
        run('exact-header-assumption',header_args,b'<table><tr><th>Location</th></tr><tr><td>China</td></tr></table>',expected=[dict(location='China')])
        run('changed-header-refused',header_args,b'<table><tr><th>Population</th></tr><tr><td>China</td></tr></table>',failure=3)
        run('normalized-boundaries',['extract','--stdin','--select','dl','--read','text'],b'<dl><dt>Name</dt><dd>Alice</dd></dl>',expected=['Name Alice'])
        run('missing-row-sibling',group,b'<table><tr class=entry><td>T</td></tr></table>',failure=3)
        run('overlapping-row-scope',group,b'<table><tr class=entry><td>T</td></tr><tr class=entry><td>T2</td></tr></table>',failure=3)
        run('ambiguous-code-language',['extract','--stdin','--select','pre','--read','markdown'],b'<pre class="language-rust language-python">x</pre>',failure=3)
        run('attribute-empty',['extract','--stdin','--select','a','--read','attr:href'],b'<a href=""></a>',expected=[''])
        run('raw-empty',['extract','--stdin','--select','p','--raw'],b'<p></p>',raw=b'')
        run('retired-source-slice',['extract','--stdin','--start','A','--end','B','--raw'],'Aé\r\nB'.encode(),failure=2)
        for label,arguments,source,code in [
            ('ambiguous',['extract','--stdin','--select','p'],b'<p>A</p><p>B</p>',3),
            ('missing',['extract','--stdin','--select','p'],b'<b>A</b>',3),
            ('missing-attribute',['extract','--stdin','--select','a','--read','attr:href'],b'<a>A</a>',3),
            ('invalid-css',['extract','--stdin','--select','['],b'<p>A</p>',2),
            ('invalid-survey-limit',['inspect','--stdin','--limit','0'],b'<p>A</p>',2),
            ('unknown-field-exclusion',['extract','--stdin','--select','tr','--field','country','td','text','--field-exclude','UNDECLARED_SECRET','sup'],b'<tr><td>China</td></tr>',2),
            ('invalid-utf8',['extract','--stdin','--select','p'],b'<p>\xff</p>',5),
            ('retired-url',['extract','--url','http://invalid.test/SYNTHETIC_SECRET','--select','p'],None,2),
            ('retired-encoding',['extract','--stdin','--select','p','--encoding','utf-16le'],b'',2),
            ('retired-projection',['extract','--stdin','--select','p','--projection','document_text'],b'',2),
            ('retired-cursor',['inspect','--stdin','--select','p','--cursor','old'],b'',2),
            ('unknown-option',['--SYNTHETIC_SECRET'],None,2)]:
            result=run(label,arguments,source,failure=code);rows[-1]['passed'] &= b'SYNTHETIC_SECRET' not in result.stderr
        plan={'version':wire_version,'select':'article','match':'all','fields':{
            'title':{'select':'h2'},'price':{'select':'.price'},
            'absent':{'select':'.missing','match':'optional'},'empty':{'select':'.missing','match':'all','min':0}}}
        path=root/'records.plan.json';path.write_text(json.dumps(plan),encoding="utf-8")
        source=root/'source.html';source.write_text('<article><h2>First é</h2><p class="price">10</p></article><article><h2>Second</h2><p class="price">20</p></article>',encoding="utf-8")
        expected=[dict(title='First é',price='10',absent=None,empty=[]),dict(title='Second',price='20',absent=None,empty=[])]
        command=['extract','--file',str(source),'--plan',str(path)]
        first=run('direct-records',command,expected=expected)
        run('ordinary-files-reproduction',command,raw=first.stdout)
        run('record-raw-refused',command+['--raw'],failure=2)
        published=root/'data.json';run('file-data',command+['--output',str(published)],raw=b'')
        rows[-1]['passed'] &= json.loads(published.read_bytes())==expected
        run('no-overwrite',command+['--output',str(published)],failure=5)
        run('input-output-collision',command+['--output',str(source),'--overwrite'],failure=2)
        inspection=run('scoped-inspection',['inspect','--stdin','--select','p','--samples','2'],b'<p>A</p><p>B</p><p>C</p>')
        value=json.loads(inspection.stdout);rows[-1]['passed'] &= value['count']==3 and len(value['samples'])==2 and not value['samples_complete']
        outline_source=b'<table id=population><tr><th>Location</th><th>Population</th></tr><tr><td>India</td><td>1</td></tr><tr><td>China</td><td>2</td></tr></table>'
        outline=run('group-outline',['inspect','--stdin'],outline_source)
        value=json.loads(outline.stdout);group=value['groups'][0]
        rows[-1]['passed'] &= value['group_count']==1 and value['groups_complete']
        rows[-1]['passed'] &= group['selector']=='#population tr' and group['count']==3 and group['table']['headers']==['Location','Population'] and group['table']['data_rows']==2
        sample_source=b'<main id=content class="docs main" data-secret=private><table><tr><td>China</td><td>17.3%</td></tr></table></main>'
        run('combined-inspection',['inspect','--stdin','--select','main'],sample_source,expected=dict(count=1,samples=[dict(tag='main',id='content',classes=['docs','main'],identifiers_complete=True,attributes=['class','data-secret','id'],attributes_complete=True,text='China 17.3%',text_complete=True)],samples_complete=True))
        for version in range(1,wire_version):
            old=dict(plan,version=version);path.write_text(json.dumps(old),encoding="utf-8");run(f'unsupported-wire-{version}',['extract','--stdin','--plan',str(path)],b'<article></article>',failure=2)
        simple={'version':wire_version,'select':'#amount','expect':[{'select':'#label','equals':'Cost'}]}
        path.write_text(json.dumps(simple),encoding="utf-8");run('original-context-guard',['extract','--stdin','--plan',str(path)],b'<b id="label">Cost</b><p id="amount">180</p>',expected=['180'])
        run('changed-context-refused',['extract','--stdin','--plan',str(path)],b'<b id="label">Other</b><p id="amount">180</p>',failure=3)
        simple['limits']={'max_work':1};path.write_text(json.dumps(simple),encoding="utf-8");run('shared-budget-refused',['extract','--stdin','--plan',str(path)],b'<p id="amount">180</p>',failure=4)
    unchanged=hashlib.sha256(binary.read_bytes()).hexdigest()==before
    evidence=dict(schema='htmlcut.native-reliability-matrix',version=4,binary_sha256=before,binary_unchanged=unchanged,rows=rows,passed=unchanged and all(row['passed'] for row in rows))
    args.output.write_text(json.dumps(evidence,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(cases=len(rows),passed=sum(row['passed'] for row in rows),binary_unchanged=unchanged)))
    if not evidence['passed']:raise SystemExit('Native current-contract assertions failed; inspect retained matrix')

if __name__=='__main__':main()
