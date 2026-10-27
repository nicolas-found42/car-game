"""Public JSON diagnostic behavior, including legacy evidence and damaged streams."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from diagnosis_rules import PhysicsRules
from unittest.mock import patch
import diagnosis_incident


class DiagnosisTests(unittest.TestCase):
    def invoke(self, *args):
        result = subprocess.run(['python3', 'tools/diagnose.py', *map(str, args)], capture_output=True, text=True, check=False)
        return result.returncode, json.loads(result.stdout)

    def test_invalid_arguments_and_missing_roots_are_machine_readable(self):
        code, result=self.invoke('timeline','missing','--stream','private')
        self.assertEqual(code,2)
        self.assertEqual(result['error'],'invalid_arguments')
        code,result=self.invoke('search','--root','missing-diagnosis-root')
        self.assertEqual(code,1)
        self.assertEqual(result['errors'][0]['code'],'root_unavailable')

    def test_timeline_is_bounded_and_reports_corruption(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'physics.jsonl').write_text('{"tick":1}\ninvalid\n{"tick":2}\n{"tick":3}\n')
            code, result = self.invoke('timeline', root, '--limit', '1')
            self.assertEqual(code, 1)
            self.assertTrue(result['truncated'])
            self.assertEqual(len(result['records']), 1)
            self.assertEqual(result['errors'][0]['code'], 'invalid_record')

    def test_search_finds_a_fleet_member_and_marks_missing_health_unknown(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            run = root / 'run';run.mkdir();(run / 'cars').mkdir()
            (run / 'manifest.json').write_text(json.dumps({'car':'350z','run_id':'test','mode':'fleet'}))
            (run / 'cars/gt500.json').write_text('{}')
            code, result = self.invoke('search', '--root', root, '--car', 'gt500')
            self.assertEqual(code, 0)
            self.assertEqual(len(result['runs']), 1)
            self.assertEqual(result['runs'][0]['recording_status'], 'unknown')

    def test_legacy_hunting_is_detected_without_inventing_missing_decisions(self):
        rules = PhysicsRules()
        for tick in range(30):
            rules.observe({'tick':tick,'car':'nsx','gear':1+tick%2,'grounded':4})
        self.assertEqual(rules.transitions, 29)
        self.assertEqual(rules.findings[0]['reason'], 'gear_hunting')
        self.assertFalse(rules.findings[0]['evidence']['historical_decisions_available'])
        self.assertNotIn('zero_drive_demand', [f['reason'] for f in rules.findings])

    def test_expected_reset_does_not_become_a_contact_loss_incident(self):
        rules=PhysicsRules()
        rules.observe({'tick':0,'car':'mini','gear':1,'grounded':4,'reset_count':0})
        for tick in range(1,30):
            rules.observe({'tick':tick,'car':'mini','gear':1,'grounded':0,'reset_count':1})
        self.assertEqual(rules.findings,[])

    def test_frozen_incident_is_queryable_without_full_session_streams(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);window=root/'incidents/0001';window.mkdir(parents=True)
            (window/'incident.json').write_text('{"reason":"contact_loss","coverage":{"post_complete":false}}')
            row={'stream':'physics','tick':25,'recorder_seq':40,'record':{'car':'mini','tick':25,'grounded':0}}
            (window/'incident-history.jsonl').write_text(json.dumps(row)+'\n')
            code,result=self.invoke('timeline',root,'--incident','1','--car','mini')
            self.assertEqual(code,0)
            self.assertEqual(result['records'][0]['_incident_recorder_seq'],40)
            self.assertFalse(result['incident']['coverage']['post_complete'])
            row['tick']=26
            (window/'incident-history.jsonl').write_text(json.dumps(row)+'\n')
            code,result=self.invoke('timeline',root,'--incident','1')
            self.assertEqual(code,1)
            self.assertEqual(result['errors'][0]['code'],'incident_tick_mismatch')

    def test_compare_detects_first_physics_difference_and_ignores_added_telemetry(self):
        with tempfile.TemporaryDirectory() as temporary:
            left=Path(temporary)/'left';right=Path(temporary)/'right';left.mkdir();right.mkdir()
            fields='step tick actor_id time car position rotation linear_velocity angular_velocity gear rpm speed slip steering input ray_misses grounded wheel_lengths wheel_loads drive_demands longitudinal_forces reset_count'.split()
            row=dict.fromkeys(fields,0);row.update(tick=1,gear=1)
            (left/'physics.jsonl').write_text(json.dumps(row)+'\n')
            (right/'physics.jsonl').write_text(json.dumps({**row,'decisions':{}})+'\n')
            self.assertTrue(self.invoke('compare',left,right)[1]['equal'])
            (right/'physics.jsonl').write_text(json.dumps({**row,'gear':2})+'\n')
            self.assertEqual(self.invoke('compare',left,right)[1]['different_fields'], ['gear'])
            row['rpm']=0.0
            (left/'physics.jsonl').write_text(json.dumps(row)+'\n')
            (right/'physics.jsonl').write_text(json.dumps({**row,'rpm':-0.0})+'\n')
            self.assertEqual(self.invoke('compare',left,right)[1]['different_fields'], ['rpm'])

    def test_current_trace_compares_without_preservation_only_force_arrays(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fields = 'step tick actor_id time car position rotation linear_velocity angular_velocity gear rpm speed slip steering input ray_misses grounded wheel_lengths wheel_loads reset_count'.split()
            (root / 'physics.jsonl').write_text(json.dumps(dict.fromkeys(fields, 0)) + '\n')
            code, result = self.invoke('compare', root, root)
            self.assertEqual(code, 0)
            self.assertTrue(result['equal'])
            self.assertEqual(result['unavailable_fields'], ['drive_demands', 'longitudinal_forces'])

    def test_actor_histories_do_not_invent_or_hide_gear_hunting(self):
        steady, hunting = PhysicsRules(), PhysicsRules()
        for tick in range(30):
            for actor, gear in [('first', 1), ('second', 2)]:
                steady.observe({'tick': tick, 'actor_id': actor, 'car': 'same', 'gear': gear})
            hunting.observe({'tick': tick, 'actor_id': 'first', 'car': 'one', 'gear': 1 + tick % 2})
            hunting.observe({'tick': tick, 'actor_id': 'second', 'car': 'two', 'gear': 2})
        self.assertEqual(steady.findings, [])
        self.assertEqual(hunting.findings[0]['actor_id'], 'first')
        self.assertEqual(hunting.transitions, 29)

    def test_incident_reports_causal_boundaries_and_unknowns_without_mutation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            run = root / 'case'; run.mkdir()
            (root / 'index.jsonl').write_text(json.dumps({'event':'start','run_id':'case'}) + '\n')
            (run / 'manifest.json').write_text(json.dumps({'run_id':'case','mode':'scenario','car':'350z'}))
            (run / 'failure.json').write_text(json.dumps({'reason':'wall-time-limit'}))
            rows = [{'kind':'scenario.capture_requested','tick':20,'render_frame':30,
                     'actor_id':'player-vehicle','correlation_id':'scenario:capture','fields':{}}]
            events = run / 'events.jsonl'; events.write_text(''.join(json.dumps(r)+'\n' for r in rows))
            before = events.read_bytes()
            code, result = self.invoke('incident', 'case', '--root', root)
            self.assertEqual(code, 1)
            self.assertEqual(result['status'], 'failed')
            self.assertEqual(result['spans'][0]['last_observed_boundary'], 'scenario.capture_requested')
            self.assertIn('missing callback', result['unknowns'][1])
            self.assertEqual(events.read_bytes(), before)
            code, listed = self.invoke('search', '--root', root)
            self.assertEqual(code, 0)
            self.assertEqual(listed['runs'][0]['run_id'], 'case')

    def test_vehicle_filter_matches_event_fields(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            row = {'tick': 5, 'kind': 'simulation.completed', 'fields': {'vehicle_id': 'pilot-350z'}}
            (root / 'events.jsonl').write_text(json.dumps(row) + '\n')
            code, result = self.invoke('timeline', root, '--stream', 'events', '--vehicle-id', 'pilot-350z')
            self.assertEqual(code, 0)
            self.assertEqual(result['records'], [row])

    def test_invalid_envelope_reports_corruption_instead_of_crashing(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'events.jsonl').write_text('{"tick":1,"envelope":"bad"}\n')
            code, result = self.invoke('timeline', root, '--stream', 'events', '--actor', 'player')
            self.assertEqual(code, 1)
            self.assertEqual(result['errors'][0]['code'], 'invalid_record')

    def test_invalid_incident_envelope_reports_corruption(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); window = root / 'incidents/0001'; window.mkdir(parents=True)
            (window / 'incident.json').write_text('{}')
            row = {'stream':'events','tick':1,'recorder_seq':1,'record':{'tick':1,'envelope':'bad'}}
            (window / 'incident-history.jsonl').write_text(json.dumps(row)+'\n')
            code, result = self.invoke('timeline', root, '--stream', 'events', '--incident', '1', '--actor', 'player')
            self.assertEqual(code, 1)
            self.assertEqual(result['errors'][0]['code'], 'invalid_incident_record')

    def test_truncated_capture_span_is_unknown_not_incomplete(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'manifest.json').write_text(json.dumps({'run_id':'r','ended_at':'now','exit':{'code':0}}))
            rows = [{'kind':'scenario.capture_requested','correlation_id':'capture','tick':1},
                    {'kind':'scenario.capture_saved','correlation_id':'capture','tick':2}]
            (root / 'events.jsonl').write_text(''.join(json.dumps(row)+'\n' for row in rows))
            with patch.object(diagnosis_incident, 'EVENT_LIMIT', 1), patch.object(diagnosis_incident, 'inspect_health', return_value={'ok':True,'status':'healthy'}):
                result = diagnosis_incident.incident(root)
            self.assertEqual(result['status'], 'complete')
            self.assertEqual(result['spans'][0]['status'], 'unknown_truncated')
            self.assertEqual(result['errors'][0]['code'], 'event_scan_limit')

    def test_replay_invocation_preserves_recorded_car(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'manifest.json').write_text(json.dumps({'run_id':'r','car':'nsx','args':['--reference-physics','--seed','42','--render-disabled']}))
            (root / 'events.jsonl').write_text('')
            (root / 'input.jsonl').write_text('{}\n')
            _, result = self.invoke('incident', root)
            command = result['reproduction']['command']
            self.assertIn('--car nsx --replay-input', command)
            self.assertIn('--reference-physics --seed 42 --render-disabled', command)

    def test_scenario_reproduction_preserves_reference_and_fast_mode(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'manifest.json').write_text(json.dumps({'run_id':'r','args':['--reference-physics','--fast-scenario','--seed','42']}))
            (root / 'scenario.json').write_text(json.dumps({'definition':{},'render_backend':'disabled'}))
            (root / 'scenario-definition.json').write_text('{}')
            (root / 'events.jsonl').write_text('')
            _, result = self.invoke('incident', root)
            command = result['reproduction']['command']
            self.assertIn('--reference-physics --seed 42 --render-disabled --fast-scenario', command)

    def test_nonobject_manifest_is_a_versioned_json_error(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'manifest.json').write_text('[]')
            code, result = self.invoke('analyze', root)
            self.assertEqual(code, 2)
            self.assertEqual(result['schema_version'], 1)
            self.assertEqual(result['error'], 'ValueError')


    def test_nonobject_event_fields_are_located_corrupt_records(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'events.jsonl').write_text('{"tick":1,"fields":[]}\n')
            code, result = self.invoke('timeline', root, '--stream', 'events')
            self.assertEqual(code, 1)
            self.assertEqual(result['errors'][0]['code'], 'invalid_record')
            self.assertEqual(result['errors'][0]['line'], 1)



if __name__ == '__main__':
    unittest.main()
