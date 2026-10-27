"""Expanded dry-road fleet protocol: fixed inputs plus shared checkpoint routes."""
import json
import math
import hashlib
from pathlib import Path

from handling_protocol import CASES, drive, scenario as original_scenario

ROOT = Path(__file__).resolve().parents[1]
EXTRA = ['launch-full', 'launch-partial', 'coast-down', 'brake-progressive', 'stop-relaunch',
         'reverse-recover', 'handbrake-left', 'handbrake-right', 'trail-brake-left',
         'trail-brake-right', 'slalom-pulses']
ROUTES = ['curve-forward-slow', 'curve-forward-fast', 'curve-reverse-slow', 'curve-reverse-fast',
          'urban-left', 'urban-right', 'lane-change', 'lane-change-reverse']
ALL_CASES = list(CASES) + EXTRA + ROUTES


def route_plan(case):
    geometry = json.loads((ROOT / 'docs/verification/scenario-courses/courses.json').read_text())
    assert hashlib.sha256((ROOT/geometry['source']).read_bytes()).hexdigest() == geometry['source_sha256'], 'Re-export courses after changing city geometry'
    if case.startswith('curve-'):
        direction = 'reverse' if 'reverse' in case else 'forward'
        return geometry['curve_' + direction], 14. if case.endswith('fast') else 8.
    if case.startswith('urban-'):
        side = -1 if case.endswith('left') else 1
        points = [[0., 200.], [0., 140.]]
        points += [[side * (10.-10.*math.cos(i*math.pi/24)), 122.-10.*math.sin(i*math.pi/24)] for i in range(13)]
        return points + [[side*40., 112.], [side*130., 112.]], 7.
    points = [[-300.,458.],[-270.,458.],[-245.,462.],[-220.,462.],[-195.,454.],[-170.,454.],[-145.,458.],[-110.,458.]]
    return (points[::-1] if case.endswith('reverse') else points), 10.


def extra_commands(case):
    commands = [drive(240, throttle=1.)]
    end = 1200
    if case == 'launch-full':
        end = 1440
    elif case == 'launch-partial':
        commands = [drive(240, throttle=.3)]; end = 1200
    elif case == 'coast-down':
        commands += [drive(720)]; end = 1440
    elif case == 'brake-progressive':
        commands += [drive(960, brake=.3), drive(1200, brake=1.)]; end = 1680
    elif case == 'stop-relaunch':
        commands += [drive(720, brake=1.), drive(1200, throttle=1.)]; end = 1680
    elif case == 'reverse-recover':
        commands = [drive(240, brake=1.), drive(600, throttle=1.)]; end = 1200
    elif case.startswith('handbrake-'):
        sign = -1 if case.endswith('left') else 1
        commands += [{'tick':600,'command':{'action':'drive','throttle':0.,'brake':0.,'steer':sign*.5,'handbrake':True}},
                     drive(660,brake=1.,steer=-sign*.2), drive(1020)]; end = 1080
    elif case.startswith('trail-brake-'):
        sign = -1 if case.endswith('left') else 1
        commands += [drive(600,brake=.5,steer=sign*.5),drive(780,brake=1.)]; end = 1080
    elif case == 'slalom-pulses':
        commands += [drive(t,throttle=.35,steer=.25*(-1)**i) for i,t in enumerate(range(600,1080,60))]
        commands += [drive(1080,brake=1.)]; end = 1320
    return commands,end


def scenario(car, case):
    if case in CASES:
        return original_scenario(car, case)
    if case in ROUTES:
        points,speed = route_plan(case)
        delta = [points[1][i]-points[0][i] for i in [0,1]]
        yaw = math.atan2(-delta[0],-delta[1])
        initial = {'car':car,'position':[points[0][0],.62,points[0][1]],'rotation_y':yaw}
        end = 5400 if case.startswith('urban') else 4800
        commands = []
        assertions = [{'tick':end,'kind':'route_completed','max_cross_track_m':7.}]
        route = {'points':points,'speed_mps':speed}
    else:
        commands,end = extra_commands(case)
        initial = {'car':car,'position':[-300.,.62,458.],'rotation_y':-math.pi/2}
        assertions = [{'tick':end,'kind':'displacement_at_least','meters':8.}]
        route = None
    assertions += [{'tick':end,'kind':'selected_car','car':car},{'tick':end,'kind':'reset_count','count':0}]
    result = {'schema_version':1,'name':f'calibration-{case}-{car}','scene':'driving','seed':17,
              'initial':initial,'commands':commands,'assertions':assertions,'max_ticks':end+1,
              'max_wall_ms':120000,'readiness_timeout_ms':10000,'required_capture':False}
    if route:
        result['route'] = route
        if case.startswith('curve-'):
            result['course'] = 'northeast_curve'
    return result
