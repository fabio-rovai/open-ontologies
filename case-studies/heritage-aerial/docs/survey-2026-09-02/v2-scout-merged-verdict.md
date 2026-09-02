# V2 collection scout: merged verdict (2 September 2026)

Two independent scouts ran the same 15-candidate brief: a 15-agent Claude
fleet (v2-collection-scout-claude.json, one agent errored on NARA) and a
single Kimi k3 session using live curl probes (v2-collection-scout.json,
v2-collection-scout.md).

Agreement: France (IGN WFS, 4.9M records, encoded sortie entity), Denmark
(KB COP REST), Spain (CNIG GeoJSON), England (Historic England ArcGIS),
Australia (GA ArcGIS), New Zealand (LINZ Koordinates, api-key), BC Canada
(WFS) all HIGH on both; Ireland LOW on both (account-gated); commercial
exemplar LOW on both.

Divergences (grade only, no contradictions): NARA rated HIGH by Kimi (the
Claude agent errored; fills the military-archive slot), Norway/Sweden/
Netherlands HIGH by Claude vs MEDIUM by Kimi, NOAA HIGH by Kimi vs MEDIUM
by Claude. Kimi assessed UConn MAGIC (high) where Claude assessed PennPilot
(high).

Net: with the existing three collections, 14 to 16 viable collections across
roughly 12 countries and at least 9 distinct catalogue architectures
(custom REST, CKAN, ArcGIS FeatureServer, ArcGIS MapServer, OGC WFS, KB COP
OpenSearch, Koordinates, STAC API, static GeoJSON, Delving aggregator).
The v2 headline is achievable. Benchmark task draft: v2-benchmark-tasks.json
(30 tasks with per-representation hypotheses, unverified).
