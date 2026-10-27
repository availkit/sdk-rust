import json,re,pathlib,shutil
dst=pathlib.Path(__file__).resolve().parent
for d in ('src','tests','examples'): (dst/d).mkdir(parents=True,exist_ok=True)
schemas=json.loads((dst/'src/schemas.json').read_text())
ops=json.loads((dst/'src/operations.json').read_text())
for name,v in [('schemas',schemas),('operations',ops)]: (dst/f'src/{name}.json').write_text(json.dumps(v,indent=2)+'\n')
(dst/'VERSION').write_text('0.1.1\n')
def snake(s): return re.sub(r'(?<!^)(?=[A-Z])','_',s).lower()
models=['//! Wire types for the maintained stable contract. Validate before any network send.','use serde::{Serialize, Deserialize};']
done=set()
def typ(v,name):
 if '$ref' in v:return v['$ref'].split('/')[-1]
 if 'anyOf' in v:return 'Option<'+typ(next(x for x in v['anyOf'] if x.get('type')!='null'),name)+'>'
 t=v.get('type')
 if isinstance(t,list):return 'Option<'+typ(dict(v,type=next(x for x in t if x!='null')),name)+'>'
 if 'enum' in v and all(isinstance(x,str) for x in v['enum']):
  if name not in done:
   done.add(name); models.extend(['#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]',f'pub enum {name} {{']+[f'    #[serde(rename = {json.dumps(x)})] {"".join(p.capitalize() for p in re.split(r"[._-]",x))},' for x in v['enum']]+['}'])
  return name
 if t=='object':gen(name,v);return name
 if t=='array': return 'Vec<'+typ(v['items'],name+'Item')+'>'
 if 'const' in v:return 'bool' if isinstance(v['const'],bool) else 'u64'
 return {'string':'String','integer':'u64','boolean':'bool'}.get(t,'String')
def gen(name,v):
 if name in done:return
 done.add(name);fields=[]
 for k,p in v.get('properties',{}).items():
  t=typ(p,name+k[0].upper()+k[1:]); optional=k not in v.get('required',[])
  if optional and not t.startswith('Option<'):t='Option<'+t+'>'
  a=f'    #[serde(rename = {json.dumps(k)}'+(', default, skip_serializing_if = "Option::is_none"' if optional else '')+')]'
  fields.extend([a,f'    pub {snake(k) if k!="type" else "kind"}: {t},'])
 models.extend(['#[derive(Clone, '+('' if name in ('WebhookCreated','Customer','CustomerInput') else 'Debug, ')+'PartialEq, Serialize, Deserialize)]','#[serde(deny_unknown_fields)]',f'pub struct {name} {{',*fields,'}'])
 if name in ('WebhookCreated','Customer','CustomerInput'): models.append(f"impl std::fmt::Debug for {name} {{ fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{ f.write_str(\"AvailKit {name} (private)\") }} }}")
for n,v in schemas.items():gen(n,v)
(dst/'src/models.rs').write_text('\n'.join(models)+'\n')
names={'setHours':'replace_hours','updateResource':'replace_resource','updateOffering':'replace_offering','rescheduleBooking':'reschedule_booking','getPublicPage':'get_page','listPublicOfferings':'list_offerings','publicAvailability':'availability','bookPublic':'book','getPublicBooking':'get_booking','cancelPublicBooking':'cancel','reschedulePublicBooking':'reschedule'}
replays={'book','cancel','rescheduleBooking','bookPublic','cancelPublicBooking','reschedulePublicBooking'}
lines=['// Generated stable operation methods; regenerate with python3 generate.py.','use crate::*;','use crate::models::*;']
for public,cl in [(False,'Client'),(True,'PublicClient')]:
 lines.append(f'impl {cl} {{')
 for op,s in ops.items():
  if ('/public/' in s['path'])!=public:continue
  name=names.get(op,snake(op)); ids=re.findall(r'\{([^}]+)\}',s['path'])[1:]
  params=['&self','cancel: &CancellationToken']; rel=s['path'].split('}',1)[1]; fm=[]
  for id in ids:
   n=snake(id);params.append(n+': &str');fm.append(f'identifier({n})?;');rel=rel.replace('{'+id+'}','{'+n+'}')
  inp=s['input'];out=s['output']['$ref'].split('/')[-1]
  body='None'
  if inp:
   if '$ref' in inp and inp['$ref'].split('/')[-1] not in ('Empty','WebhookDisable'):
    t=inp['$ref'].split('/')[-1];params.append(f'body: &{t}');body='Some(serde_json::to_value(body).map_err(|_| Error::input("invalid_input"))?)'
   else:body='Some(serde_json::json!({}))'
  query='vec![]'
  if op in ('availability','publicAvailability'):
   params+=['range: &AvailabilityQuery'];query='range.pairs()?'
  elif op=='events':params+=['page: &EventQuery'];query='page.pairs()?'
  elif op.startswith('list'):
   params+=['page: &PageQuery'];query='page.pairs()?'
  headers='vec![]'
  if public and ('Booking' in op or op=='bookPublic'):
   params+=['booking_token: &str'];fm.append('capability(booking_token)?;');headers='vec![("Authorization".into(), format!("Booking {booking_token}"))]'
   if op=='bookPublic':params+=['contact_token: &str'];fm.append('capability(contact_token)?;');headers='vec![("Authorization".into(), format!("Booking {booking_token}")), ("X-AvailKit-Contact-Token".into(), contact_token.into())]'
  fm.append(f'let path = format!({json.dumps(rel)});')
  if op in replays:
   prep_params=[p for p in params if p!='cancel: &CancellationToken']+['idempotency_key: &str']
   lines+= [f'    pub fn prepare_{name}({", ".join(prep_params)}) -> Result<Intent<{out}>> {{',*['        '+x for x in fm],f'        self.inner.prepare({json.dumps(op)}, path, {body}.unwrap(), idempotency_key, {headers})','    }']
   params+=['idempotency_key: &str'];args=[p.split(':')[0] for p in prep_params if p!='&self']
   lines +=[f'    pub async fn {name}({", ".join(params)}) -> Result<{out}> {{',f'        self.execute(&self.prepare_{name}({", ".join(args)})?, cancel).await','    }']
  else:
   lines +=[f'    pub async fn {name}({", ".join(params)}) -> Result<{out}> {{',*['        '+x for x in fm],f'        self.inner.call({json.dumps(op)}, path, {body}, {query}, {headers}, None, cancel).await','    }']
 lines+=['}']
(dst/'src/methods.rs').write_text('\n'.join(lines)+'\n')
if pathlib.Path(__file__).resolve() != (dst/'generate.py').resolve(): shutil.copyfile(__file__,dst/'generate.py')
