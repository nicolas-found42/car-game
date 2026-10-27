"""Conservative symptom rules; findings are evidence, not automatic retuning advice."""
from collections import Counter, deque


def decision_explanation(reason, row):
    decision = row.get('decisions')
    if not isinstance(decision, dict):
        return {'basis': 'legacy_state_only', 'mechanism': 'decision_details_unavailable'}
    if reason == 'gear_hunting':
        mechanism = ('repeated_shifts_interrupt_engine_demand'
                     if decision.get('drive_cutoff') == 'shift_in_progress' else 'gear_selection_oscillation')
        return {'basis': 'recorded_decision', 'mechanism': mechanism}
    if reason == 'zero_drive_demand':
        return {'basis': 'recorded_decision', 'mechanism': decision.get('drive_cutoff', 'unknown'),
                'engine_demand_n': decision.get('engine_demand_n')}
    contacts = Counter(wheel.get('contact', 'unknown') for wheel in decision.get('wheels', []))
    mechanism = ('suspension_queries_skipped_for_body_orientation' if contacts.get('body_inverted') == 4
                 else 'suspension_raycast_support_absent_or_rejected')
    return {'basis': 'recorded_decision', 'mechanism': mechanism, 'contact_reasons': dict(contacts)}


class ActorRules:
    def __init__(self):
        self.previous = None
        self.changes = deque(maxlen=120)
        self.seen = set()
        self.transitions = 0
        self.unsupported = 0
        self.supported = False
        self.zero = 0
        self.findings = []
        self.samples = 0

    def finding(self, reason, row, detail):
        key = (reason, row.get('car'))
        if key in self.seen:
            return
        self.seen.add(key)
        if len(self.findings) < 1000:
            self.findings.append({'reason': reason, 'car': row.get('car'), 'actor_id': row.get('actor_id'),
                                  'vehicle_id': row.get('vehicle_id'), 'tick': row['tick'],
                                  'confidence': 'observed_symptom', 'evidence': detail,
                                  'explanation': decision_explanation(reason, row)})

    def observe(self, row):
        self.samples += 1
        old = self.previous
        if old and (old.get('car') != row.get('car') or old.get('reset_count') != row.get('reset_count')):
            self.changes.clear()
            self.supported = False
            self.unsupported = self.zero = 0
            old = None
        if old and min(old.get('gear', 0), row.get('gear', 0)) > 0 and old['gear'] != row['gear']:
            self.changes.append(row['tick'])
            self.transitions += 1
        while self.changes and row['tick'] - self.changes[0] >= 120:
            self.changes.popleft()
        if len(self.changes) >= 8:
            self.finding('gear_hunting', row, {'transitions_in_120_ticks': len(self.changes),
                         'decision': row.get('decisions'), 'historical_decisions_available': 'decisions' in row})
        self.supported |= row.get('grounded') == 4
        self.unsupported = self.unsupported + 1 if self.supported and row.get('grounded') == 0 else 0
        if self.unsupported == 12:
            self.finding('contact_loss', row, {'unsupported_ticks': 12, 'decisions': row.get('decisions')})
        self.observe_drive(row)
        self.previous = row

    def observe_drive(self, row):
        control = row.get('input', {})
        demands = row.get('drive_demands')
        zero = (control.get('throttle', 0) > 0.5 and control.get('brake', 0) < 0.01
                and row.get('grounded', 0) > 0 and isinstance(demands, list)
                and bool(demands) and all(abs(value) < 0.001 for value in demands))
        self.zero = self.zero + 1 if zero else 0
        if self.zero == 120:
            self.finding('zero_drive_demand', row, {'duration_ticks': 120, 'decisions': row.get('decisions')})


class PhysicsRules:
    def __init__(self):
        self.actors = {}

    def observe(self, row):
        identity = row.get('actor_id') or row.get('vehicle_id') or ('legacy', row.get('car'))
        self.actors.setdefault(identity, ActorRules()).observe(row)

    @property
    def findings(self):
        return sorted((finding for rules in self.actors.values() for finding in rules.findings),
                      key=lambda finding: finding['tick'])[:1000]

    @property
    def samples(self):
        return sum(rules.samples for rules in self.actors.values())

    @property
    def transitions(self):
        return sum(rules.transitions for rules in self.actors.values())


def clock_gap(previous, row):
    if not previous or min(previous.get('elapsed_ns', -1), row.get('elapsed_ns', -1)) < 0:
        return None
    elapsed = (row['elapsed_ns'] - previous['elapsed_ns']) / 1e9
    ticks = row.get('tick', 0) - previous.get('tick', 0)
    if elapsed > 1 and elapsed - max(0, ticks) / 120 > 0.5:
        return {'reason': 'clock_gap', 'tick': row.get('tick'), 'confidence': 'observed_gap_cause_unknown',
                'evidence': {'elapsed_s': elapsed, 'tick_advance': ticks,
                             'previous_kind': previous.get('kind'), 'kind': row.get('kind')}}
    return None
