#!/usr/bin/env python3
"""Build tests/fixtures/*.json from the per-property examples in the checked-in spec."""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
SPEC = json.loads((ROOT / "specs" / "twelvedata.json").read_text())
SCHEMAS = SPEC["components"]["schemas"]

FIXTURES = {
    "quote": "GetQuote_200_response",
    "time_series": "GetTimeSeries_200_response",
    "price": "GetPrice_200_response",
    "eod": "GetEod_200_response",
    "exchange_rate": "GetExchangeRate_200_response",
    "profile": "GetProfile_200_response",
    "statistics": "GetStatistics_200_response",
    "price_target": "GetPriceTarget_200_response",
    "earnings": "GetEarnings_200_response",
    "symbol_search": "GetSymbolSearch_200_response",
    "api_usage": "GetApiUsage_200_response",
}


def resolve(schema):
    if "$ref" in schema:
        return SCHEMAS[schema["$ref"].rsplit("/", 1)[1]]
    return schema


def synthesise(schema):
    schema = resolve(schema)
    if schema.get("examples"):
        return schema["examples"][0]
    if "example" in schema:
        return schema["example"]
    if "properties" in schema:
        return {name: synthesise(prop) for name, prop in schema["properties"].items()}
    if schema.get("type") == "array":
        return [synthesise(schema.get("items", {}))]
    for key in ("oneOf", "anyOf"):
        if key in schema:
            return synthesise(schema[key][0])
    if "allOf" in schema:
        merged = {}
        for part in schema["allOf"]:
            value = synthesise(part)
            if isinstance(value, dict):
                merged.update(value)
        return merged
    raise ValueError(f"no example for {json.dumps(schema)[:120]}")


def main():
    out = ROOT / "tests" / "fixtures"
    out.mkdir(parents=True, exist_ok=True)
    for name, schema in FIXTURES.items():
        body = synthesise(SCHEMAS[schema])
        (out / f"{name}.json").write_text(json.dumps(body, indent=2) + "\n")


if __name__ == "__main__":
    main()
