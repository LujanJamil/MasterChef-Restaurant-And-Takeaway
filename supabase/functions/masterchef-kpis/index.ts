import { createClient } from 'https://esm.sh/@supabase/supabase-js@2.57.4'

const cors = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type, x-device-id, x-device-token',
  'Access-Control-Allow-Methods': 'POST, OPTIONS',
}

const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { ...cors, 'Content-Type': 'application/json' } })
const encoder = new TextEncoder()
async function sha256Base64(value: string) {
  const digest = await crypto.subtle.digest('SHA-256', encoder.encode(value))
  let binary = ''
  for (const byte of new Uint8Array(digest)) binary += String.fromCharCode(byte)
  return btoa(binary)
}

Deno.serve(async (req) => {
  if (req.method === 'OPTIONS') return new Response('ok', { headers: cors })
  if (req.method !== 'POST') return json({ error: 'METHOD_NOT_ALLOWED' }, 405)

  const deviceId = req.headers.get('x-device-id')
  const deviceToken = req.headers.get('x-device-token')
  if (!deviceId || !deviceToken) return json({ error: 'DEVICE_AUTH_REQUIRED' }, 401)

  const supabase = createClient(Deno.env.get('SUPABASE_URL')!, Deno.env.get('SUPABASE_SERVICE_ROLE_KEY')!, { auth: { persistSession: false } })
  const { data: device, error: deviceError } = await supabase.from('sync_devices').select('device_id,status').eq('device_id', deviceId).maybeSingle()
  if (deviceError || !device || device.status !== 'ACTIVE') return json({ error: 'DEVICE_NOT_AUTHORIZED' }, 403)

  const expected = await sha256Base64(deviceToken)
  const { data: credential } = await supabase.from('sync_device_credentials').select('token_hash').eq('device_id', deviceId).maybeSingle()
  if (!credential || credential.token_hash !== expected) return json({ error: 'INVALID_DEVICE_CREDENTIAL' }, 401)

  const body = await req.json().catch(() => null)
  if (!body || body.action !== 'daily_kpis') return json({ error: 'INVALID_ACTION' }, 400)

  const { data, error } = await supabase.rpc('ceo_daily_kpis')
  if (error) return json({ error: 'KPI_QUERY_FAILED', detail: error.message }, 500)

  await supabase.from('sync_devices').update({ last_seen_at: new Date().toISOString(), updated_at: new Date().toISOString() }).eq('device_id', deviceId)
  return json({ kpis: data, server_time: new Date().toISOString() })
})
