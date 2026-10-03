#!/usr/bin/env python3
"""Current contract controls against the supplied real native executable, without acquisition."""
import argparse, hashlib, json, subprocess, tempfile
from pathlib import Path


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();binary=args.binary.resolve();before=hashlib.sha256(binary.read_bytes()).hexdigest();rows=[]
    def run(label,arguments,data=None,expected=None,raw=None,failure=None):
        result=subprocess.run([str(binary),*arguments],input=data,capture_output=True,timeout=20)
        passed=result.returncode==(failure or 0)
        if failure:passed &= result.stdout==b'' and json.loads(result.stderr)['version']==3
        else:
            passed &= result.stderr==b''
            if raw is not None:passed &= result.stdout==raw
            if expected is not None:passed &= json.loads(result.stdout)==expected
        rows.append(dict(label=label,arguments=arguments,exit_code=result.returncode,passed=passed,
                         stdout=result.stdout.decode('utf-8'),stderr=result.stderr.decode('utf-8')))
        return result
    with tempfile.TemporaryDirectory(prefix='htmlcut-native-contract-') as directory:
        root=Path(directory)
        for label,source,css,projection,expected in [
            ('literal-hidden','<p>A<span hidden>B</span><template>T</template></p>','p','dom_text',['ABT']),
            ('reading-hidden','<p>A<span hidden>B</span><template>T</template><script>S</script></p>','p','markdown',['AB']),
            ('reference-label','<p>Before <a class="reference internal" href="next">IMPORTANT</a> after.</p>','p','markdown',['Before [IMPORTANT](<next>) after.']),
            ('definition-blocks','<dl><dt>Name</dt><dd>Alice</dd></dl>','dl','markdown',['Name\n\nAlice']),
            ('source-ordinals','<ol start="3"><li>A</li><li value="8">B</li></ol>','ol','markdown',['- 3\\. A\n- 8\\. B']),
            ('table-cells','<table><tr><th>A</th><td>B</td></tr></table>','table','markdown',['-\n  - **A**\n  - B']),
            ('pre-fragment','<pre>OUTSIDE<code id="x"> x\n</code>TAIL</pre>','#x','markdown',['```\n x\n\n```']),
            ('outer-html','<P data-x="">V</P>','p','outer_html',['<p data-x="">V</p>'])]:
            run(label,['extract','--stdin','--css',css,'--projection',projection],source.encode(),expected=expected)
        run('attribute-empty',['extract','--stdin','--css','a','--attribute','href'],b'<a href=""></a>',expected=[''])
        run('raw-empty',['extract','--stdin','--css','p','--raw'],b'<p></p>',raw=b'')
        run('exact-source-slice',['extract','--stdin','--start','A','--end','B','--raw'],'Aé\r\nB'.encode(),raw='é\r\n'.encode())
        for label,arguments,source,code in [
            ('ambiguous',['extract','--stdin','--css','p'],b'<p>A</p><p>B</p>',3),
            ('missing',['extract','--stdin','--css','p'],b'<b>A</b>',3),
            ('missing-attribute',['extract','--stdin','--css','a','--attribute','href'],b'<a>A</a>',3),
            ('invalid-css',['extract','--stdin','--css','['],b'<p>A</p>',2),
            ('invalid-utf8',['extract','--stdin','--css','p'],b'<p>\xff</p>',5),
            ('retired-url',['extract','--url','http://invalid.test/SYNTHETIC_SECRET','--css','p'],None,2),
            ('retired-encoding',['extract','--stdin','--css','p','--encoding','utf-16le'],b'',2),
            ('retired-projection',['extract','--stdin','--css','p','--projection','document_text'],b'',2),
            ('retired-cursor',['inspect','--stdin','--css','p','--cursor','old'],b'',2),
            ('unknown-option',['--SYNTHETIC_SECRET'],None,2)]:
            result=run(label,arguments,source,failure=code);rows[-1]['passed'] &= b'SYNTHETIC_SECRET' not in result.stderr
        plan={'schema':'htmlcut.extraction.plan','version':3,'strategy':{'kind':'css','selector':'article'},
              'selection':{'kind':'all'},'projection':{'kind':'records','fields':[
                {'name':'title','selector':'h2'},{'name':'price','selector':'.price'},
                {'name':'absent','selector':'.missing','selection':{'kind':'optional'}},
                {'name':'empty','selector':'.missing','selection':{'kind':'all','min':0}}]}}
        path=root/'records.plan.json';path.write_text(json.dumps(plan),encoding="utf-8")
        source=root/'source.html';source.write_text('<article><h2>First é</h2><p class="price">10</p></article><article><h2>Second</h2><p class="price">20</p></article>',encoding="utf-8")
        expected=[dict(title='First é',price='10',absent=None,empty=[]),dict(title='Second',price='20',absent=None,empty=[])]
        bundle=root/'snapshot.htmlcut.tar';run('direct-records-and-bundle',['extract','--file',str(source),'--plan',str(path),'--bundle',str(bundle)],expected=expected)
        moved=root/'moved.htmlcut.tar';bundle.rename(moved);source.unlink();path.unlink()
        run('moved-input-independent-replay',['run',str(moved)],expected=expected)
        receipt=root/'receipt.json'
        replay=run('separate-record-receipt',['run',str(moved),'--receipt',str(receipt)],expected=expected)
        canonical=json.dumps(expected,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
        evidence=json.loads(receipt.read_bytes())
        rows[-1]['passed'] &= replay.stdout==canonical+b'\n' and evidence['schema']=='htmlcut.extraction.receipt'
        rows[-1]['passed'] &= evidence['version']==3 and evidence['semantics']==3 and evidence['data_kind']=='records'
        rows[-1]['passed'] &= evidence['candidate_count']==2 and evidence['selected_count']==2
        rows[-1]['passed'] &= evidence['data_sha256']==hashlib.sha256(canonical).hexdigest()
        rows[-1]['passed'] &= evidence['fields']==[
            dict(field_index=1,candidate_count=2,projected_count=2,absent_count=0),
            dict(field_index=2,candidate_count=2,projected_count=2,absent_count=0),
            dict(field_index=3,candidate_count=0,projected_count=0,absent_count=2),
            dict(field_index=4,candidate_count=0,projected_count=0,absent_count=0)]
        rows[-1]['passed'] &= b'First' not in receipt.read_bytes() and str(root).encode() not in receipt.read_bytes()
        run('record-raw-refused',['run',str(moved),'--raw'],failure=2)
        published=root/'data.json';run('file-data',['run',str(moved),'--output',str(published)],raw=b'')
        rows[-1]['passed'] &= json.loads(published.read_bytes())==expected
        run('no-overwrite',['run',str(moved),'--output',str(published)],failure=5)
        run('bundle-collision',['run',str(moved),'--output',str(moved),'--overwrite'],failure=2)
        inspection=run('scoped-inspection',['inspect','--stdin','--css','p','--samples','2'],b'<p>A</p><p>B</p><p>C</p>')
        value=json.loads(inspection.stdout);rows[-1]['passed'] &= value['count']==3 and len(value['samples'])==2 and not value['samples_complete']
        for version in [1,2,4]:
            old=dict(plan,version=version);path.write_text(json.dumps(old),encoding="utf-8");run(f'unsupported-wire-{version}',['extract','--stdin','--plan',str(path)],b'<article></article>',failure=2)
        simple={'schema':'htmlcut.extraction.plan','version':3,'strategy':{'kind':'css','selector':'#amount'},
                'guards':[{'scope':'document','selector':'#label','min':1,'max':1,'read':{'kind':'dom_text'},'predicate':{'kind':'exact','value':'Cost'}}]}
        path.write_text(json.dumps(simple),encoding="utf-8");run('original-context-guard',['extract','--stdin','--plan',str(path)],b'<b id="label">Cost</b><p id="amount">180</p>',expected=['180'])
        run('changed-context-refused',['extract','--stdin','--plan',str(path)],b'<b id="label">Other</b><p id="amount">180</p>',failure=3)
        simple['limits']={'max_work':1};path.write_text(json.dumps(simple),encoding="utf-8");run('shared-budget-refused',['extract','--stdin','--plan',str(path)],b'<p id="amount">180</p>',failure=4)
    unchanged=hashlib.sha256(binary.read_bytes()).hexdigest()==before
    evidence=dict(schema='htmlcut.native-reliability-matrix',version=3,binary_sha256=before,binary_unchanged=unchanged,rows=rows,passed=unchanged and all(row['passed'] for row in rows))
    args.output.write_text(json.dumps(evidence,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(cases=len(rows),passed=sum(row['passed'] for row in rows),binary_unchanged=unchanged)))
    if not evidence['passed']:raise SystemExit('Native current-contract assertions failed; inspect retained matrix')

if __name__=='__main__':main()
