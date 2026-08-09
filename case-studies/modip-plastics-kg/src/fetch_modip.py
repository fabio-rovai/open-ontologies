"""
Download the full Museum of Design in Plastics (MoDiP) object record set from
the Museum Data Service (MDS) extract API.

The MDS extract endpoint is token-paginated: each response carries a `resume`
token for the next page of 100 records. A token is obtained (free) from the
declaration form on any MDS object-search results page:
    https://museumdata.uk/object-search/?collection[]=Museum of Design in Plastics
    -> "Get an API token"

Pass the token via the MDS_TOKEN environment variable. It is never written to
disk by this script and must not be committed.

    MDS_TOKEN="<token>" python3 src/fetch_modip.py

Output: data/raw/modip_records.json  (list of MDS @document records)

Data © the contributing museum (MoDiP / Arts University Bournemouth), licensed
CC BY 4.0 via the Museum Data Service. Attribution is retained per record in the
License / License Url fields.
"""
import json, os, sys, time, urllib.parse, urllib.request

BASE = "https://mds-data-2.ciim.k-int.com/api/v1/extract"
ROOT = os.path.dirname(os.path.dirname(__file__))
OUT = os.path.join(ROOT, "data", "raw", "modip_records.json")


def get(resume):
    url = BASE + "?" + urllib.parse.urlencode({"resume": resume})
    req = urllib.request.Request(url, headers={"User-Agent": "modip-kg/1.0"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


def main():
    token = os.environ.get("MDS_TOKEN")
    if not token:
        sys.exit("Set MDS_TOKEN (free from museumdata.uk object-search 'Get an API token').")
    records, resume, page = [], token, 0
    while True:
        for attempt in range(4):
            try:
                d = get(resume)
                break
            except Exception as e:
                print(f"  retry {attempt} page {page}: {e}", flush=True)
                time.sleep(3)
        else:
            sys.exit(f"failed at page {page}")
        records.extend(d.get("data", []))
        page += 1
        st = d.get("stats", {})
        print(f"page {page}: total={len(records)} remaining={st.get('remaining')}", flush=True)
        if not d.get("has_next"):
            break
        resume = d.get("resume")
        if not resume:
            break
        time.sleep(0.3)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    json.dump(records, open(OUT, "w"))
    print(f"DONE {len(records)} records -> {OUT}")


if __name__ == "__main__":
    main()
