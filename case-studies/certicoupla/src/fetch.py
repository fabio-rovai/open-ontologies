import requests, json, time
rows=[]
url="https://oqmd.org/optimade/structures"
params={"response_fields":"chemical_formula_reduced,_oqmd_delta_e,_oqmd_band_gap,_oqmd_stability","page_limit":200}
for i in range(20):
    try:
        r=requests.get(url, params=params, timeout=40).json()
    except Exception as e:
        print("stop:", e); break
    for d in r.get("data",[]):
        a=d.get("attributes",{})
        rows.append({"name":a.get("chemical_formula_reduced"),
                     "delta_e":a.get("_oqmd_delta_e"),"band_gap":a.get("_oqmd_band_gap"),
                     "stability":a.get("_oqmd_stability")})
    nxt=(r.get("links") or {}).get("next")
    json.dump(rows, open("data/oqmd.json","w"))
    print(f"page {i+1}: total {len(rows)}", flush=True)
    if not nxt: break
    # next is a full URL
    url = nxt if isinstance(nxt,str) else nxt.get("href")
    params=None
    time.sleep(0.2)
clean=[x for x in rows if all(isinstance(x.get(k),(int,float)) for k in ("delta_e","band_gap","stability"))]
json.dump(clean, open("data/oqmd.json","w"))
print("FINAL clean:", len(clean))
