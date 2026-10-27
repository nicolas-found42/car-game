-- Open with: sqlite3 docs/vehicle-reference/corpus.sqlite
-- Field names and units preserve the research sources; inspect notes before comparing.

-- 1. Which real cars have models in more than one source game?
SELECT car.id, car.name, car.year_json,
       count(DISTINCT counterpart.game) AS games,
       group_concat(DISTINCT counterpart.game) AS game_list
FROM car JOIN counterpart ON counterpart.car_id = car.id
GROUP BY car.id HAVING count(DISTINCT counterpart.game) > 1
ORDER BY games DESC, car.id;

-- 2. Which assets have a named model/year/trim match (still review market/equipment)?
SELECT c.name, g.game_name, g.catalog_name, g.match_status, g.match_note
FROM car c JOIN counterpart g ON g.car_id=c.id
WHERE g.match_status='exact_named_model_year_trim';

-- 3. Source coverage and unresolved items, without multiplying facts across game assets.
SELECT c.id,c.name,
       (SELECT count(*) FROM real_fact f WHERE f.car_id=c.id) AS real_facts,
       (SELECT count(*) FROM source s WHERE s.car_id=c.id) AS sources,
       (SELECT count(*) FROM unknown u WHERE u.car_id=c.id) AS unresolved
FROM car c ORDER BY real_facts,c.id;

-- 4. Candidate visual wheelbase mismatches. Each result still needs identity/source review:
-- multiple real facts can be alternatives; model pivots can be exporter reconstructions.
SELECT c.id,c.name,g.game_name,g.asset_code,
       json_extract(f.value_json,'$') AS reference_mm,
       1000.0*json_extract(m.value_json,'$') AS model_mm,
       f.note,s.url
FROM real_fact f JOIN car c ON c.id=f.car_id
JOIN counterpart g ON g.car_id=c.id
JOIN model_measurement m ON m.asset_code=g.asset_code
JOIN source s ON s.id=f.source_id
WHERE f.field='wheelbase' AND f.unit='mm'
  AND json_type(f.value_json) IN ('integer','real')
  AND m.field='wheelbase_from_pivots_m'
  AND abs(1000.0*json_extract(m.value_json,'$')/json_extract(f.value_json,'$')-1)>0.02
ORDER BY c.id,g.game_name;

-- 5. Find sourced drivetrain/differential, mass, tire and gearing facts for an AWD profile.
SELECT f.field,f.value_json,f.unit,f.note,s.title,s.url
FROM real_fact f JOIN source s ON s.id=f.source_id
WHERE f.car_id='j13'
ORDER BY f.field;

-- 6. Inspect native GT4 brake, grip, weight distribution, gearing and spring encodings.
-- These are not SI constants; consult DATA-DICTIONARY.md.
SELECT g.catalog_name,f.json_path,f.value_json,f.encoding
FROM game_fact f JOIN counterpart g USING(asset_code)
WHERE g.asset_code='GT4_SUBR0003'
 AND (f.json_path LIKE '%BRAKE:%'
      OR f.json_path LIKE '%percentageF%'
      OR f.json_path LIKE '%performance%'
      OR f.json_path LIKE '%springrate%'
      OR f.json_path LIKE '%GEAR:%');

-- 7. List acceleration/braking/skidpad/top-speed evidence with original test conditions.
SELECT c.id,c.name,f.field,f.value_json,f.unit,f.note,s.url
FROM real_fact f JOIN car c ON c.id=f.car_id JOIN source s ON s.id=f.source_id
WHERE f.field LIKE '%0_60%' OR f.field LIKE '%0_100%'
   OR f.field LIKE '%60_0%' OR f.field LIKE '%70_0%'
   OR f.field LIKE '%skidpad%' OR f.field LIKE '%top_speed%'
ORDER BY c.id,f.field;
