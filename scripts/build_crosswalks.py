#!/usr/bin/env python3
"""Build clinical crosswalk Parquet file from open sources.

Sources:
- WHO ICD-10 codes (via open CSV mirrors)
- SNOMED-CT to ICD-10 mapping (NHS TRUD open mapping files)
- MeSH descriptors (NLM FTP)

Output: data/crosswalks.parquet with columns:
    source_code, source_system, target_code, target_system, relation, source_label, target_label

Usage:
    python scripts/build_crosswalks.py
"""

import pyarrow as pa
import pyarrow.parquet as pq

# Minimal seed data for development/testing.
# In production, fetch from WHO, NHS TRUD, NLM.
SEED_DATA = [
    ("I10", "ICD10", "38341003", "SNOMED", "equivalent", "Essential hypertension", "Hypertensive disorder"),
    ("I11", "ICD10", "64715009", "SNOMED", "equivalent", "Hypertensive heart disease", "Hypertensive heart disease"),
    ("E11", "ICD10", "44054006", "SNOMED", "equivalent", "Type 2 diabetes mellitus", "Diabetes mellitus type 2"),
    ("J45", "ICD10", "195967001", "SNOMED", "equivalent", "Asthma", "Asthma"),
    ("C34", "ICD10", "254637007", "SNOMED", "equivalent", "Malignant neoplasm of bronchus and lung", "Non-small cell lung cancer"),
    ("I10", "ICD10", "D003062", "MeSH", "related", "Essential hypertension", "Hypertension"),
    ("E11", "ICD10", "D003924", "MeSH", "related", "Type 2 diabetes mellitus", "Diabetes Mellitus, Type 2"),
    ("38341003", "SNOMED", "D006973", "MeSH", "related", "Hypertensive disorder", "Hypertension"),
]


def build():
    table = pa.table({
        "source_code": [r[0] for r in SEED_DATA],
        "source_system": [r[1] for r in SEED_DATA],
        "target_code": [r[2] for r in SEED_DATA],
        "target_system": [r[3] for r in SEED_DATA],
        "relation": [r[4] for r in SEED_DATA],
        "source_label": [r[5] for r in SEED_DATA],
        "target_label": [r[6] for r in SEED_DATA],
    })
    pq.write_table(table, "data/crosswalks.parquet")
    print(f"Written {len(SEED_DATA)} rows to data/crosswalks.parquet")


if __name__ == "__main__":
    build()
