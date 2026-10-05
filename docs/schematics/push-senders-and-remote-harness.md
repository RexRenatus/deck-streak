# Push senders against recording fakes, and the remote harness

SPEC-343, ADR-354. Every `path:line` here was read at DeckStreak `dev`
`1eec0870e67e801fc3aa279318219d6986116ec7`. Eight diagrams: the push data flow, one APNs call and
the outcome of an answer, the provider token's life, the remote harness's input flow with its two
state machines, and the CI jobs that judge the delivery.

## 1. The push data flow

What exists today is the router and the bot's port (`crates/notifications/src/router.rs:69` makes
the `Pass` only the router holds; `crates/notifications/src/transport.rs:84` is the bot's port).
This spike builds the `deck-streak-push` box and its fakes. The dashed edge is #640's: the
transport per surface, and its join in the composition root. The dotted platforms are the owner's
first device session (#629).

```mermaid
flowchart LR
  event["an occasion: a celebration or a nudge"] --> router
  policy["notifications-policy.json: surfaces bot and mini-app, budgets, quiet hours"] --> router
  router["the one router (notifications): dedupe, budgets, quiet hours, lapse, the claim before the send"]
  router -- "today, with the router's Pass" --> bot["BotTransport, the bot's port"]
  router -. "issue 640: a transport per surface, joined in wiring.rs" .-> callers

  subgraph push["deck-streak-push, an adapter on the kernel, not composed"]
    callers["deliver(device or subscription, notification)"]
    apns["ApnsSender: path, headers, JSON body, at most 4096 bytes"]
    web["WebPushSender: RFC 8291 aes128gcm, at most 3993 bytes of plaintext"]
    list["the push-service list: an endpoint's origin must be on it"]
    signer["the ES256 signer: one P-256 key per sender, parsed once"]
    tokens["the provider token: reused, minted at 45 minutes"]
    client["the HTTP client: one deadline, a bounded error read, no redirect"]
    outcome["Sent: Delivered, Gone, Rejected, RetryLater, Failed"]
    callers --> apns
    callers --> web
    web --> list
    apns --> tokens --> signer
    web -- "a VAPID token per message" --> signer
    apns -- "HTTP/2 only" --> client
    web -- "HTTP/1.1 or HTTP/2" --> client
    client --> outcome
  end

  clock["the kernel's clock: iat, apns-expiration, exp"] --> push
  key["the signing key's PEM text: a test's, made in memory, or the credential loader's in production (issue 640)"] --> signer

  client -- "CI: h2c, loopback" --> fakeapns["APNs fake: records, verifies the JWT, answers as scripted"]
  client -- "CI: HTTP/1.1, loopback" --> fakeweb["push service fake: records, decrypts by its own RFC 8291, verifies VAPID"]
  client -. "issue 629: TLS" .-> realapns["Apple's push service"]
  client -. "issue 629: TLS" .-> realweb["a browser's push service"]
  outcome -. "issue 640: the router's ledger" .-> router
```

What each fake records and what the tests read from it:

| fake | records | verifies itself | the tests assert |
|---|---|---|---|
| APNs | method, path, every header, the body, the order of requests | the bearer token parses as a JWT and its ES256 signature verifies with the test's public key | `POST /3/device/<token>`; `apns-push-type`, `apns-priority`, `apns-topic`, `apns-expiration`, `apns-collapse-id`; the body's JSON value; the JWT's header and claims exactly; the request count per answer; which origin a device reached |
| push service | method, path, every header, the raw body | the body decrypts with the test subscription's private key and secret; the VAPID JWT verifies with `k` | `content-encoding: aes128gcm`; the plaintext's JSON value; `TTL`, `Urgency`, `Topic`; `vapid t=…, k=…` with `aud`, `exp`, `sub`; the request count per answer |

## 2. One APNs call

```mermaid
sequenceDiagram
  participant C as the caller (a test, later the router)
  participant S as ApnsSender
  participant T as the provider token
  participant F as the APNs fake
  C->>S: deliver(device, notification)
  S->>S: body over 4096 bytes? then Rejected(TooLarge), no request
  S->>T: the current token (minted if none, or 45 minutes old)
  S->>F: POST /3/device/token at the device's environment origin
  F-->>S: status, and a JSON reason on an error
  alt 403 ExpiredProviderToken and the token is 20 minutes old or more
    S->>T: mint a new token
    S->>F: POST once more, the same notification
    F-->>S: status
  end
  S-->>C: one Sent, logged by its outcome and status only
```

The outcome of an answer, for both senders (SPEC-343 R4):

```mermaid
flowchart TD
  answer["the answer"] --> ok{"2xx"}
  ok -- yes --> delivered["Delivered"]
  ok -- no --> gone{"APNs 410, web push 404 or 410"}
  gone -- yes --> g["Gone, with Apple's timestamp on APNs"]
  gone -- no --> big{"413"}
  big -- yes --> tl["Rejected(TooLarge)"]
  big -- no --> auth{"APNs 403, web push 401 or 403"}
  auth -- yes --> pt["Rejected(ProviderToken), after the one resend where it applies"]
  auth -- no --> later{"429 or 5xx"}
  later -- yes --> rl["RetryLater, after Retry-After when sent"]
  later -- no --> bad{"another 4xx"}
  bad -- "APNs BadDeviceToken or DeviceTokenNotForTopic" --> tok["Rejected(Token)"]
  bad -- "any other" --> req["Rejected(Request)"]
  bad -- "not 4xx: 1xx, 3xx, an error, the deadline" --> failed["Failed"]
```

## 3. The provider token's life

Apple accepts a token up to an hour old and refuses a new one more often than every 20 minutes on
one connection. The sender reuses one token and mints the next at 45 minutes.

```mermaid
stateDiagram-v2
  [*] --> none
  none --> young: the first send mints
  young --> aging: 20 minutes pass
  aging --> stale: 45 minutes pass
  stale --> young: the next send mints
  young --> young: 403 ExpiredProviderToken answers Rejected(ProviderToken), nothing minted
  aging --> young: 403 ExpiredProviderToken mints, and the notification is sent once more
```

The age check and the mint happen under one lock with no await inside it, and a refused token is
replaced only while it is still the current one, so calls refused together mint one token between
them and each resends with it (SPEC-343 R3).

## 4. The remote harness

### Input flow

```mermaid
flowchart LR
  frame["each animation frame while a gamepad is connected"] --> snap["getGamepads(): a snapshot per index"]
  snap --> std{"mapping is standard"}
  std -- no --> raw["shown raw, fires nothing"]
  std -- yes --> edges["rising edges against the last snapshot, the first fires nothing, the stick with hysteresis"]
  key["keydown"] --> filter{"repeat, composing, a control, Alt, an unmapped Control or Command"}
  filter -- yes --> rawkey["shown raw, fires nothing"]
  filter -- no --> keymap["section 7's keys"]
  edges --> side{"the review side"}
  keymap --> side
  side -- "question: show answer only" --> act["an action"]
  side -- "answer: grades, undo, bury, flag, replay" --> act
  act --> log["the log: time, source, raw, action, visibility, lock, the last 200"]
  raw --> log
  rawkey --> log
  conn["gamepadconnected, gamepaddisconnected"] --> holder
  vis["visibilitychange"] --> holder
  review["the review button"] --> holder
  holder["the wake lock holder"] --> log
```

### A gamepad's connection

```mermaid
stateDiagram-v2
  [*] --> absent
  absent --> baseline: gamepadconnected, or a snapshot names it
  baseline --> reading: the first snapshot is kept, nothing fires
  reading --> reading: a frame fires the rising edges of a standard mapping
  reading --> absent: gamepaddisconnected, or a snapshot no longer names it
  baseline --> absent: gamepaddisconnected
```

A browser withholds `gamepadconnected` until a button or an axis on the gamepad is used, so the
snapshot also counts as a connection.

### The wake lock holder

The condition is: a review is open, a gamepad is connected, and the page is visible. `want` is its
rise, `unwant` its fall; `granted` and `denied` answer the one request in flight; `released` is the
browser's release of the lock the holder holds. Seven states, five events, thirty-five cells, each
examined by A32.

```mermaid
stateDiagram-v2
  [*] --> off
  [*] --> unsupported: no navigator.wakeLock
  off --> requesting: want, request the screen lock
  requesting --> held: granted
  requesting --> refused: denied, its name recorded
  requesting --> cancelling: unwant
  cancelling --> requesting: want
  cancelling --> off: granted, release it at once
  cancelling --> off: denied
  held --> off: unwant, release it
  held --> released: released by the browser
  released --> requesting: want, request again
  released --> off: unwant
  refused --> off: unwant
```

| state | want | unwant | granted | denied | released |
|---|---|---|---|---|---|
| off | requesting, request | off | off | off | off |
| requesting | requesting | cancelling | held | refused | requesting |
| cancelling | requesting | cancelling | off, release | off | cancelling |
| held | held | off, release | held | held | released |
| released | requesting, request | off | released | released | released |
| refused | refused | off | refused | refused | refused |
| unsupported | unsupported | unsupported | unsupported | unsupported | unsupported |

A `granted`, `denied` or `released` that reaches a state with no request in flight, or comes from
a lock the holder no longer holds, changes nothing. When the page is hidden the browser releases
the lock and the condition falls, in either order; both orders end in `off`, and the next `want`,
on visible, requests again (A30).

## 5. The CI jobs

No job is added. The rows below are the jobs of `.github/workflows/ci.yml` at the commit above
that judge this delivery.

```mermaid
flowchart LR
  rust["rust (line 30): fmt, clippy, test, doctest, audit-rust"] --> ci
  web["web (line 201): svelte-check, vitest, build, the accessibility audit"] --> ci
  hygiene["hygiene (line 268): python, scrub, secrets"] --> ci
  plan["mutation-plan (line 344)"] --> mrust["mutation-rust (line 415): cargo-mutants over the diff"]
  plan --> mrows["mutation-rows (line 543): the band's rows"]
  mrust --> verdict["mutation-verdict (line 614)"]
  mrows --> verdict
  plan --> verdict
  mweb["mutation-web (line 672): StrykerJS over each changed web file, break 100"] --> ci
  verdict --> ci["ci (line 837): every job"]
```

| job | what it runs for this delivery |
|---|---|
| `rust` | the push crate's tests (A1 to A19), the censuses (A20 to A22), clippy with warnings denied, `cargo deny` over the new crates |
| `web` | the harness's tests (A23 to A34), and the accessibility audit, which visits `/remote` in both colour schemes |
| `hygiene` | the secrets scan, which finds no key in the tree |
| `mutation-rust` | cargo-mutants over the push crate's changed lines |
| `mutation-rows` | the band's rows, each installed, its killer run red, and the file restored |
| `mutation-web` | StrykerJS over every changed file under `web/app/src`, the screen included, with no survivor |
