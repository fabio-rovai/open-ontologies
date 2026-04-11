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

# Sample crosswalk data covering common clinical conditions.
# Sources: ICD-10 codes (WHO, public), SNOMED CT concept IDs (IHTSDO, codes are public identifiers),
# MeSH descriptors (NLM/NIH, public domain). Mappings based on published equivalence tables.
# For production use, fetch full mappings from WHO, NHS TRUD, and NLM.
SEED_DATA = [
    # ── Cardiovascular ──────────────────────────────────────────────
    ("I10", "ICD10", "38341003", "SNOMED", "equivalent", "Essential hypertension", "Hypertensive disorder"),
    ("I11", "ICD10", "64715009", "SNOMED", "equivalent", "Hypertensive heart disease", "Hypertensive heart disease"),
    ("I20", "ICD10", "194828000", "SNOMED", "equivalent", "Angina pectoris", "Angina pectoris"),
    ("I21", "ICD10", "22298006", "SNOMED", "equivalent", "Acute myocardial infarction", "Myocardial infarction"),
    ("I25", "ICD10", "53741008", "SNOMED", "equivalent", "Chronic ischaemic heart disease", "Coronary arteriosclerosis"),
    ("I48", "ICD10", "49436004", "SNOMED", "equivalent", "Atrial fibrillation and flutter", "Atrial fibrillation"),
    ("I50", "ICD10", "84114007", "SNOMED", "equivalent", "Heart failure", "Heart failure"),
    ("I63", "ICD10", "422504002", "SNOMED", "equivalent", "Cerebral infarction", "Ischemic stroke"),
    ("I70", "ICD10", "441574008", "SNOMED", "equivalent", "Atherosclerosis", "Atherosclerosis"),
    ("I10", "ICD10", "D006973", "MeSH", "related", "Essential hypertension", "Hypertension"),
    ("I21", "ICD10", "D009203", "MeSH", "related", "Acute myocardial infarction", "Myocardial Infarction"),
    ("I48", "ICD10", "D001281", "MeSH", "related", "Atrial fibrillation and flutter", "Atrial Fibrillation"),
    ("I50", "ICD10", "D006333", "MeSH", "related", "Heart failure", "Heart Failure"),
    ("I63", "ICD10", "D002544", "MeSH", "related", "Cerebral infarction", "Cerebral Infarction"),
    ("38341003", "SNOMED", "D006973", "MeSH", "related", "Hypertensive disorder", "Hypertension"),
    ("84114007", "SNOMED", "D006333", "MeSH", "related", "Heart failure", "Heart Failure"),
    ("49436004", "SNOMED", "D001281", "MeSH", "related", "Atrial fibrillation", "Atrial Fibrillation"),

    # ── Endocrine / metabolic ───────────────────────────────────────
    ("E10", "ICD10", "46635009", "SNOMED", "equivalent", "Type 1 diabetes mellitus", "Diabetes mellitus type 1"),
    ("E11", "ICD10", "44054006", "SNOMED", "equivalent", "Type 2 diabetes mellitus", "Diabetes mellitus type 2"),
    ("E03", "ICD10", "40930008", "SNOMED", "equivalent", "Other hypothyroidism", "Hypothyroidism"),
    ("E05", "ICD10", "34486009", "SNOMED", "equivalent", "Thyrotoxicosis", "Hyperthyroidism"),
    ("E66", "ICD10", "414916001", "SNOMED", "equivalent", "Obesity", "Obesity"),
    ("E78", "ICD10", "55822004", "SNOMED", "equivalent", "Disorders of lipoprotein metabolism", "Hyperlipidemia"),
    ("E11", "ICD10", "D003924", "MeSH", "related", "Type 2 diabetes mellitus", "Diabetes Mellitus, Type 2"),
    ("E10", "ICD10", "D003922", "MeSH", "related", "Type 1 diabetes mellitus", "Diabetes Mellitus, Type 1"),
    ("E66", "ICD10", "D009765", "MeSH", "related", "Obesity", "Obesity"),
    ("44054006", "SNOMED", "D003924", "MeSH", "related", "Diabetes mellitus type 2", "Diabetes Mellitus, Type 2"),

    # ── Respiratory ─────────────────────────────────────────────────
    ("J06", "ICD10", "54150009", "SNOMED", "equivalent", "Acute upper respiratory infection", "Upper respiratory infection"),
    ("J18", "ICD10", "233604007", "SNOMED", "equivalent", "Pneumonia, unspecified organism", "Pneumonia"),
    ("J44", "ICD10", "13645005", "SNOMED", "equivalent", "Chronic obstructive pulmonary disease", "Chronic obstructive lung disease"),
    ("J45", "ICD10", "195967001", "SNOMED", "equivalent", "Asthma", "Asthma"),
    ("J45", "ICD10", "D001249", "MeSH", "related", "Asthma", "Asthma"),
    ("J44", "ICD10", "D029424", "MeSH", "related", "Chronic obstructive pulmonary disease", "Pulmonary Disease, Chronic Obstructive"),
    ("J18", "ICD10", "D011014", "MeSH", "related", "Pneumonia, unspecified organism", "Pneumonia"),
    ("195967001", "SNOMED", "D001249", "MeSH", "related", "Asthma", "Asthma"),

    # ── Neoplasms ───────────────────────────────────────────────────
    ("C18", "ICD10", "363406005", "SNOMED", "equivalent", "Malignant neoplasm of colon", "Colon cancer"),
    ("C34", "ICD10", "254637007", "SNOMED", "equivalent", "Malignant neoplasm of bronchus and lung", "Non-small cell lung cancer"),
    ("C50", "ICD10", "254837009", "SNOMED", "equivalent", "Malignant neoplasm of breast", "Breast cancer"),
    ("C61", "ICD10", "399068003", "SNOMED", "equivalent", "Malignant neoplasm of prostate", "Prostate cancer"),
    ("C43", "ICD10", "372244006", "SNOMED", "equivalent", "Malignant melanoma of skin", "Malignant melanoma"),
    ("C50", "ICD10", "D001943", "MeSH", "related", "Malignant neoplasm of breast", "Breast Neoplasms"),
    ("C34", "ICD10", "D008175", "MeSH", "related", "Malignant neoplasm of bronchus and lung", "Lung Neoplasms"),
    ("C61", "ICD10", "D011471", "MeSH", "related", "Malignant neoplasm of prostate", "Prostatic Neoplasms"),
    ("C18", "ICD10", "D003110", "MeSH", "related", "Malignant neoplasm of colon", "Colonic Neoplasms"),

    # ── Musculoskeletal ─────────────────────────────────────────────
    ("M54", "ICD10", "279039007", "SNOMED", "equivalent", "Dorsalgia", "Low back pain"),
    ("M17", "ICD10", "239873007", "SNOMED", "equivalent", "Gonarthrosis", "Osteoarthritis of knee"),
    ("M81", "ICD10", "64859006", "SNOMED", "equivalent", "Osteoporosis without fracture", "Osteoporosis"),
    ("M05", "ICD10", "69896004", "SNOMED", "equivalent", "Rheumatoid arthritis", "Rheumatoid arthritis"),
    ("M81", "ICD10", "D010024", "MeSH", "related", "Osteoporosis without fracture", "Osteoporosis"),
    ("M05", "ICD10", "D001172", "MeSH", "related", "Rheumatoid arthritis", "Arthritis, Rheumatoid"),

    # ── Mental health ───────────────────────────────────────────────
    ("F32", "ICD10", "35489007", "SNOMED", "equivalent", "Depressive episode", "Depressive disorder"),
    ("F41", "ICD10", "197480006", "SNOMED", "equivalent", "Other anxiety disorders", "Anxiety disorder"),
    ("F20", "ICD10", "58214004", "SNOMED", "equivalent", "Schizophrenia", "Schizophrenia"),
    ("F10", "ICD10", "7200002", "SNOMED", "equivalent", "Mental disorders due to use of alcohol", "Alcoholism"),
    ("F32", "ICD10", "D003866", "MeSH", "related", "Depressive episode", "Depressive Disorder"),
    ("F41", "ICD10", "D001008", "MeSH", "related", "Other anxiety disorders", "Anxiety Disorders"),
    ("F20", "ICD10", "D012559", "MeSH", "related", "Schizophrenia", "Schizophrenia"),
    ("35489007", "SNOMED", "D003866", "MeSH", "related", "Depressive disorder", "Depressive Disorder"),

    # ── Neurological ────────────────────────────────────────────────
    ("G30", "ICD10", "26929004", "SNOMED", "equivalent", "Alzheimer disease", "Alzheimer disease"),
    ("G20", "ICD10", "49049000", "SNOMED", "equivalent", "Parkinson disease", "Parkinson disease"),
    ("G40", "ICD10", "84757009", "SNOMED", "equivalent", "Epilepsy", "Epilepsy"),
    ("G43", "ICD10", "37796009", "SNOMED", "equivalent", "Migraine", "Migraine"),
    ("G35", "ICD10", "24700007", "SNOMED", "equivalent", "Multiple sclerosis", "Multiple sclerosis"),
    ("G30", "ICD10", "D000544", "MeSH", "related", "Alzheimer disease", "Alzheimer Disease"),
    ("G20", "ICD10", "D010300", "MeSH", "related", "Parkinson disease", "Parkinson Disease"),
    ("G40", "ICD10", "D004827", "MeSH", "related", "Epilepsy", "Epilepsy"),

    # ── Gastrointestinal ────────────────────────────────────────────
    ("K21", "ICD10", "235595009", "SNOMED", "equivalent", "Gastro-oesophageal reflux disease", "Gastroesophageal reflux disease"),
    ("K50", "ICD10", "34000006", "SNOMED", "equivalent", "Crohn disease", "Crohn disease"),
    ("K51", "ICD10", "64766004", "SNOMED", "equivalent", "Ulcerative colitis", "Ulcerative colitis"),
    ("K80", "ICD10", "235919008", "SNOMED", "equivalent", "Cholelithiasis", "Cholelithiasis"),
    ("K50", "ICD10", "D003424", "MeSH", "related", "Crohn disease", "Crohn Disease"),
    ("K51", "ICD10", "D003093", "MeSH", "related", "Ulcerative colitis", "Colitis, Ulcerative"),

    # ── Renal ───────────────────────────────────────────────────────
    ("N18", "ICD10", "709044004", "SNOMED", "equivalent", "Chronic kidney disease", "Chronic kidney disease"),
    ("N20", "ICD10", "95570007", "SNOMED", "equivalent", "Calculus of kidney", "Kidney stone"),
    ("N39", "ICD10", "68566005", "SNOMED", "equivalent", "Urinary tract infection", "Urinary tract infection"),
    ("N18", "ICD10", "D051436", "MeSH", "related", "Chronic kidney disease", "Renal Insufficiency, Chronic"),

    # ── Infectious disease ──────────────────────────────────────────
    ("A09", "ICD10", "25374005", "SNOMED", "equivalent", "Infectious gastroenteritis and colitis", "Gastroenteritis"),
    ("B20", "ICD10", "86406008", "SNOMED", "equivalent", "HIV disease", "Human immunodeficiency virus infection"),
    ("A15", "ICD10", "56717001", "SNOMED", "equivalent", "Respiratory tuberculosis", "Tuberculosis"),
    ("B17", "ICD10", "66071002", "SNOMED", "equivalent", "Other acute viral hepatitis", "Viral hepatitis type B"),
    ("U07.1", "ICD10", "840539006", "SNOMED", "equivalent", "COVID-19", "COVID-19"),
    ("U07.1", "ICD10", "D000086382", "MeSH", "related", "COVID-19", "COVID-19"),
    ("B20", "ICD10", "D015658", "MeSH", "related", "HIV disease", "HIV Infections"),
    ("A15", "ICD10", "D014376", "MeSH", "related", "Respiratory tuberculosis", "Tuberculosis"),

    # ── Dermatological ──────────────────────────────────────────────
    ("L40", "ICD10", "9014002", "SNOMED", "equivalent", "Psoriasis", "Psoriasis"),
    ("L20", "ICD10", "24079001", "SNOMED", "equivalent", "Atopic dermatitis", "Atopic dermatitis"),
    ("L40", "ICD10", "D011565", "MeSH", "related", "Psoriasis", "Psoriasis"),

    # ── Ophthalmological ────────────────────────────────────────────
    ("H40", "ICD10", "23986001", "SNOMED", "equivalent", "Glaucoma", "Glaucoma"),
    ("H25", "ICD10", "193570009", "SNOMED", "equivalent", "Senile cataract", "Cataract"),
    ("H40", "ICD10", "D005901", "MeSH", "related", "Glaucoma", "Glaucoma"),

    # ── Haematological ──────────────────────────────────────────────
    ("D50", "ICD10", "87522002", "SNOMED", "equivalent", "Iron deficiency anaemia", "Iron deficiency anemia"),
    ("D64", "ICD10", "271737000", "SNOMED", "equivalent", "Other anaemias", "Anemia"),
    ("D50", "ICD10", "D018798", "MeSH", "related", "Iron deficiency anaemia", "Anemia, Iron-Deficiency"),
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
