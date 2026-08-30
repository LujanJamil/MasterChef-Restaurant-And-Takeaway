import { createClient } from 'https://esm.sh/@supabase/supabase-js@2.57.4'

const allowedOrigin = Deno.env.get('MASTERCHIEF_ALLOWED_ORIGIN') ?? 'tauri://localhost'
const cors = {
  'Access-Control-Allow-Origin': allowedOrigin,
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type, x-device-id, x-device-token',
  'Access-Control-Allow-Methods': 'POST, OPTIONS',
  'Vary': 'Origin',
}
const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { ...cors, 'Content-Type': 'application/json' } })
const encoder = new TextEncoder()
async function sha256Base64(value: string) { const digest = await crypto.subtle.digest('SHA-256', encoder.encode(value)); let binary=''; for (const byte of new Uint8Array(digest)) binary += String.fromCharCode(byte); return btoa(binary) }
Deno.serve(async (req) => {
  if (req.method === 'OPTIONS') return new Response('ok', { headers: cors })
  if (req.method !== 'POST') return json({ error: 'METHOD_NOT_ALLOWED' }, 405)
  const deviceId=req.headers.get('x-device-id'), deviceToken=req.headers.get('x-device-token')
  if(!deviceId||!deviceToken)return json({error:'DEVICE_AUTH_REQUIRED'},401)
  const serviceKey=Deno.env.get('SUPABASE_SERVICE_ROLE_KEY'), url=Deno.env.get('SUPABASE_URL')
  if(!serviceKey||!url)return json({error:'SERVER_MISCONFIGURED'},500)
  const supabase=createClient(url,serviceKey,{auth:{persistSession:false}})
  const {data:device,error:deviceError}=await supabase.from('sync_devices').select('device_id,status').eq('device_id',deviceId).maybeSingle()
  if(deviceError||!device||device.status!=='ACTIVE')return json({error:'DEVICE_NOT_AUTHORIZED'},403)
  const expected=await sha256Base64(deviceToken)
  const {data:credential}=await supabase.from('sync_device_credentials').select('token_hash').eq('device_id',deviceId).maybeSingle()
  if(!credential||credential.token_hash!==expected)return json({error:'INVALID_DEVICE_CREDENTIAL'},401)
  const body=await req.json().catch(()=>null)
  if(!body||!Array.isArray(body.operations))return json({error:'INVALID_PAYLOAD'},400)
  if(body.operations.length>250)return json({error:'BATCH_TOO_LARGE'},413)
  const accepted:string[]=[], rejected:{id:string;error:string}[]=[]
  for(const op of body.operations){try{if(!op?.id||!op?.entity_type||!op?.entity_id||!op?.operation||typeof op.sequence!=='number'||!op.payload)throw new Error('INVALID_OPERATION');const {data,error}=await supabase.rpc('record_sync_operation',{p_id:op.id,p_device_id:deviceId,p_entity_type:op.entity_type,p_entity_id:op.entity_id,p_operation:op.operation,p_payload:op.payload,p_payload_hash:op.payload_hash,p_sequence:op.sequence,p_client_created_at:op.client_created_at});if(error)throw error;accepted.push(data.id)}catch(e){rejected.push({id:String(op?.id??'unknown'),error:e instanceof Error?e.message:'SYNC_REJECTED'})}}
  await supabase.from('sync_devices').update({last_seen_at:new Date().toISOString(),updated_at:new Date().toISOString()}).eq('device_id',deviceId)
  return json({accepted,rejected,server_time:new Date().toISOString()})
})
