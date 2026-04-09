# Management API

The Maverick Management API is the first administrative surface built on top of the refactored kernel.

It exists to prove that a full capability can be implemented without coupling HTTP handlers directly to SQL or bypassing kernel contracts.

## Device Endpoints

- `POST /api/v1/devices`
- `GET /api/v1/devices/:dev_eui`
- `PATCH /api/v1/devices/:dev_eui`
- `DELETE /api/v1/devices/:dev_eui`

## Boundary Formats

- `dev_eui`: hex string
- `app_eui`: hex string
- `app_key`: base64 string
- `nwk_key`: base64 string

## Example Create Payload

```json
{
  "dev_eui": "0102030405060708",
  "app_eui": "0807060504030201",
  "app_key": "AQEBAQEBAQEBAQEBAQEBAQ==",
  "nwk_key": "AgICAgICAgICAgICAgICAg==",
  "class": "ClassA"
}
```

## Error Semantics

- invalid input -> `400`
- duplicate device -> `409`
- missing device -> `404`
- infrastructure failure -> `500`

## Kernel Guarantees for This Surface

- handlers translate boundary DTOs into application commands
- use cases operate through repository and audit contracts
- device operations publish semantic events
- successful and rejected operations are recorded in `audit_log`