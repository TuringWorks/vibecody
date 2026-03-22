---
layout: page
title: "Demo 29: Regex & Encoding Tools"
permalink: /demos/regex-encoding/
nav_order: 29
parent: Demos
---


## Overview

This demo covers seven developer utility panels in VibeCody: Regex pattern tester, JWT decoder/encoder, Encoding converter, Number Base converter, Cron expression builder, Timestamp converter, and Data Generator. These tools run entirely client-side and are available in both the CLI and VibeUI.

**Time to complete:** ~12 minutes

## Prerequisites

- VibeCody installed and configured
- For VibeUI: the desktop app running (`npm run tauri dev`)

## Step-by-Step Walkthrough

### Regex Panel: Pattern Testing

#### Step 1: Test a regex pattern

```bash
vibecli repl
> /regex test --pattern "\b[A-Z][a-z]+\b" --input "Hello World from VibeCody"
```

```
Pattern: \b[A-Z][a-z]+\b
Flags: global

Matches (3):
  Match 1: "Hello"   at index 0-5
  Match 2: "World"   at index 6-11
  Match 3: "Vibe"    at index 17-21

No capture groups.
```

#### Step 2: Extract capture groups

```bash
> /regex test \
    --pattern "(\d{4})-(\d{2})-(\d{2})" \
    --input "Released on 2026-03-13 and updated 2026-03-14"
```

```
Pattern: (\d{4})-(\d{2})-(\d{2})
Flags: global

Matches (2):
  Match 1: "2026-03-13" at index 12-22
    Group 1: "2026"
    Group 2: "03"
    Group 3: "13"
  Match 2: "2026-03-14" at index 35-45
    Group 1: "2026"
    Group 2: "03"
    Group 3: "14"
```

#### Step 3: Test with flags

```bash
> /regex test \
    --pattern "error:.*" \
    --flags "gi" \
    --input "Error: file not found\nerror: permission denied"
```

```
Matches (2):
  Match 1: "Error: file not found"     at line 1
  Match 2: "error: permission denied"  at line 2
```

In VibeUI, the **Regex** panel shows match highlighting in real time as you type, with colored overlays on the input text for each match and group.


### JWT Panel: Token Inspection

#### Step 4: Decode a JWT token

```bash
> /jwt decode eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IlZpYmVDb2R5IiwiaWF0IjoxNzA5MjkxMjAwLCJleHAiOjE3MDkzNzc2MDB9.signature
```

```
Header:
  {
    "alg": "HS256",
    "typ": "JWT"
  }

Payload:
  {
    "sub": "1234567890",
    "name": "VibeCody",
    "iat": 1709291200,
    "exp": 1709377600
  }

Claims Analysis:
  Issued At:  2024-03-01T12:00:00Z
  Expires:    2024-03-02T12:00:00Z
  Status:     EXPIRED (374 days ago)
  Subject:    1234567890
```

#### Step 5: Encode a JWT

```bash
> /jwt encode \
    --payload '{"sub":"user-42","role":"admin","iat":1709291200}' \
    --secret "my-secret-key" \
    --algorithm HS256
```

```
Token:
  eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyLTQyIiwicm9sZSI6ImFkbWluIiwiaWF0IjoxNzA5MjkxMjAwfQ.abc123...
```

In VibeUI, the **JWT** panel has two sections: paste a token at the top to see its decoded header, payload, and expiration status below. A separate "Encode" tab lets you build tokens from scratch.


### Encoding Panel: Data Conversion

#### Step 6: Convert between encoding formats

```bash
> /encoding base64 encode "Hello VibeCody"
```

```
Base64: SGVsbG8gVmliZUNvZHk=
```

```bash
> /encoding base64 decode "SGVsbG8gVmliZUNvZHk="
```

```
Decoded: Hello VibeCody
```

Other encoding operations:

```bash
> /encoding url encode "hello world & foo=bar"
# Result: hello%20world%20%26%20foo%3Dbar

> /encoding hex encode "VibeCody"
# Result: 56696265436f6479

> /encoding html encode "<script>alert('xss')</script>"
# Result: &lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;
```

In VibeUI, the **Encoding** panel shows input and output side by side with a format selector dropdown (Base64, URL, Hex, HTML Entities). Encoding and decoding update in real time as you type.


### Number Base Panel: Base Conversion

#### Step 7: Convert between number bases

```bash
> /numbase convert 255
```

```
Decimal:     255
Hexadecimal: FF
Octal:       377
Binary:      11111111
```

Convert from any base:

```bash
> /numbase convert --from hex "1A3F"
```

```
Decimal:     6719
Hexadecimal: 1A3F
Octal:       15077
Binary:      1101000111111
```

In VibeUI, the **Number Base** panel has four input fields (decimal, hex, octal, binary) that update each other in real time as you type in any field.


### Cron Panel: Expression Builder

#### Step 8: Parse and preview a cron expression

```bash
> /cron parse "0 9 * * MON-FRI"
```

```
Expression: 0 9 * * MON-FRI
Description: At 09:00 AM, Monday through Friday

Next 5 runs:
  1. 2026-03-16 09:00:00 (Mon)  in 2 days
  2. 2026-03-17 09:00:00 (Tue)  in 3 days
  3. 2026-03-18 09:00:00 (Wed)  in 4 days
  4. 2026-03-19 09:00:00 (Thu)  in 5 days
  5. 2026-03-20 09:00:00 (Fri)  in 6 days

Field breakdown:
  Minute:     0
  Hour:       9
  Day(month): * (every day)
  Month:      * (every month)
  Day(week):  MON-FRI
```

Build a cron expression interactively:

```bash
> /cron build
```

```
Cron Builder (use arrow keys to select)
  Minute:      [0 ]  (0-59, */5, etc.)
  Hour:        [*/2]  (every 2 hours)
  Day(month):  [*  ]  (every day)
  Month:       [*  ]  (every month)
  Day(week):   [*  ]  (every day)

  Result: 0 */2 * * *
  Description: At minute 0 past every 2nd hour
```

In VibeUI, the **Cron** panel has dropdowns for each field plus a visual calendar preview highlighting when the job will run.


### Timestamp Panel: Epoch Converter

#### Step 9: Convert timestamps

```bash
> /timestamp now
```

```
Current Time:
  Unix (seconds):       1710331200
  Unix (milliseconds):  1710331200000
  ISO 8601:             2026-03-13T12:00:00Z
  RFC 2822:             Thu, 13 Mar 2026 12:00:00 +0000
  Local:                2026-03-13 07:00:00 EST
```

Convert an epoch timestamp:

```bash
> /timestamp convert 1609459200
```

```
Input: 1609459200 (Unix seconds)

  UTC:         2021-01-01T00:00:00Z
  US/Eastern:  2020-12-31 19:00:00 EST
  US/Pacific:  2020-12-31 16:00:00 PST
  Europe/London: 2021-01-01 00:00:00 GMT
  Asia/Tokyo:  2021-01-01 09:00:00 JST
```

Convert from a human-readable date:

```bash
> /timestamp parse "March 13, 2026 12:00 PM UTC"
```

```
Unix (seconds):       1710331200
Unix (milliseconds):  1710331200000
ISO 8601:             2026-03-13T12:00:00Z
```

In VibeUI, the **Timestamp** panel has an input field that auto-detects epoch or date strings, and displays all timezone conversions below. A timezone selector lets you add custom zones.


### Data Gen Panel: Fake Data Generation

#### Step 10: Generate fake data

```bash
> /datagen generate --type user --count 3 --format json
```

```json
[
  {
    "id": "usr_a1b2c3",
    "name": "Alice Johnson",
    "email": "alice.johnson@example.com",
    "phone": "+1-555-0142",
    "address": "742 Maple Street, Springfield, IL 62704"
  },
  {
    "id": "usr_d4e5f6",
    "name": "Bob Chen",
    "email": "bob.chen@example.net",
    "phone": "+1-555-0198",
    "address": "1234 Oak Avenue, Portland, OR 97201"
  },
  {
    "id": "usr_g7h8i9",
    "name": "Carol Diaz",
    "email": "carol.diaz@example.org",
    "phone": "+1-555-0267",
    "address": "567 Pine Road, Austin, TX 78701"
  }
]
```

Generate other data types:

```bash
> /datagen generate --type uuid --count 5
> /datagen generate --type creditcard --count 2 --format csv
> /datagen generate --type address --count 3 --locale en_GB
```

In VibeUI, the **Data Gen** panel has a type selector, count field, format dropdown (JSON, CSV, SQL INSERT), and locale picker. Click "Generate" to produce data with a "Copy to clipboard" button.

## Demo Recording

```json
{
  "meta": {
    "title": "Regex & Encoding Tools",
    "description": "Test regex patterns, decode JWTs, convert encodings, bases, cron expressions, timestamps, and generate fake data.",
    "duration_seconds": 300,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/regex test --pattern \"\\b[A-Z][a-z]+\\b\" --input \"Hello World from VibeCody\"", "delay_ms": 2000 }
      ],
      "description": "Test a regex pattern with match highlighting"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/regex test --pattern \"(\\d{4})-(\\d{2})-(\\d{2})\" --input \"Released on 2026-03-13\"", "delay_ms": 2000 }
      ],
      "description": "Extract capture groups from a date pattern"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/jwt decode eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IlZpYmVDb2R5In0.sig", "delay_ms": 2000 }
      ],
      "description": "Decode a JWT and inspect claims"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/encoding base64 encode \"Hello VibeCody\"", "delay_ms": 1500 },
        { "input": "/encoding url encode \"hello world & foo=bar\"", "delay_ms": 1500 },
        { "input": "/encoding hex encode \"VibeCody\"", "delay_ms": 1500 }
      ],
      "description": "Convert data between Base64, URL, and Hex encodings"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/numbase convert 255", "delay_ms": 1500 },
        { "input": "/numbase convert --from hex \"1A3F\"", "delay_ms": 1500 }
      ],
      "description": "Convert numbers between decimal, hex, octal, and binary"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/cron parse \"0 9 * * MON-FRI\"", "delay_ms": 2000 }
      ],
      "description": "Parse a cron expression and preview next runs"
    },
    {
      "id": 7,
      "action": "repl",
      "commands": [
        { "input": "/timestamp now", "delay_ms": 1500 },
        { "input": "/timestamp convert 1609459200", "delay_ms": 1500 }
      ],
      "description": "Convert epoch timestamps and display timezone breakdowns"
    },
    {
      "id": 8,
      "action": "repl",
      "commands": [
        { "input": "/datagen generate --type user --count 3 --format json", "delay_ms": 2000 }
      ],
      "description": "Generate fake user data in JSON format"
    },
    {
      "id": 9,
      "action": "vibeui",
      "panels": ["Regex", "JWT", "Encoding", "NumberBase", "Cron", "Timestamp", "DataGen"],
      "description": "Tour all seven developer utility panels in VibeUI",
      "delay_ms": 5000
    }
  ]
}
```

## What's Next

- [Demo 30: Notebook & Scripts](../notebook-scripts/) -- Interactive notebooks and AI-assisted scripting
- [Demo 25: SWE-bench Benchmarking](../swe-bench/) -- Benchmark your AI provider with SWE-bench
- [Demo 26: QA Validation Pipeline](../qa-validation/) -- Validate code with 8 QA agents
