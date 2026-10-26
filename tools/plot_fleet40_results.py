#!/usr/bin/env python3
"""Render inspectable fleet-wide measurements and scenario coverage figures."""
import argparse
import json
from pathlib import Path

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

ROOT=Path(__file__).resolve().parents[1]


def read(path):
    return json.loads(path.read_text())


def benchmarks(out, roster):
    data=read(out/'reference-comparison.json');rows={r['car']:r for r in data['measurements']}
    fig,axes=plt.subplots(1,2,figsize=(15,15),sharey=True)
    for ax,field,title in zip(axes,['zero_60_mph_s','brake_70_mph_0_m'],['0–60 mph (seconds)','70–0 mph (metres)']):
        values=[rows[c['id']][field] for c in roster]
        ax.barh(range(40),values,color='#486d9a',height=.68)
        for i,car in enumerate(roster):
            for ref in data['rows']:
                if ref['car']==car['id'] and ref['metric']==field and not ref['upper_bound']:
                    ax.scatter(ref['reference'],i,color='#d88c28',marker='D',s=24,zorder=3)
        ax.set_title(title);ax.grid(axis='x',alpha=.2);ax.set_axisbelow(True)
    axes[0].set_yticks(range(40),[c['name'] for c in roster],fontsize=8);axes[0].invert_yaxis()
    fig.suptitle('40 distinct car editions: production-physics measurements',x=.03,ha='left')
    fig.text(.03,.015,'Blue: two identical 120 Hz fixture repeats, no rollout. Orange: source-qualified claims/tests; related trims and test methods differ.\nOther speed endpoints and bounds remain in the source comparison table. Tire, launch and suspension estimates are not measured validation.',fontsize=9)
    fig.tight_layout(rect=[0,.045,1,.97]);fig.savefig(out/'benchmark-40.png',dpi=150);plt.close(fig)


def handling(out, roster):
    data=read(out/'results.json');cases=data['study']['cases']
    rows={(r['car'],r['case']):r for r in data['aggregate']}
    values=np.array([[rows[c['id'],case]['peak_tilt_deg'] for case in cases] for c in roster])
    fig,ax=plt.subplots(figsize=(17,15));image=ax.imshow(values,aspect='auto',cmap='YlOrRd',vmin=0)
    ax.set_yticks(range(40),[c['name'] for c in roster],fontsize=8)
    ax.set_xticks(range(len(cases)),cases,rotation=65,ha='right',fontsize=8)
    for i,car in enumerate(roster):
        for j,case in enumerate(cases):
            if not rows[car['id'],case]['scenario_passed']:ax.text(j,i,'×',ha='center',va='center',color='blue')
            elif rows[car['id'],case]['four_contact_pct'] < 100.:
                ax.text(j,i,'○',ha='center',va='center',color='cyan',fontsize=12)
    fig.colorbar(image,ax=ax,label='Peak chassis tilt (degrees)',shrink=.7)
    ax.set_title('40 cars × 24 scenarios × two repetitions — peak tilt; blue × failed assertion, cyan ○ wheel-ray contact loss')
    fig.text(.02,.012,'Tilt includes pitch, roll and map impacts over the post-launch window; deliberate reset tails are excluded. Smaller is not automatically more realistic.\nSource-backed and estimated fields remain separate. Raw support, slip, endpoint and reset outcomes are preserved in results.csv / results.json.',fontsize=9)
    fig.tight_layout(rect=[0,.045,1,.98]);fig.savefig(out/'scenario-matrix.png',dpi=150);plt.close(fig)


def curve_paths(out, roster):
    data=read(out/'results.json');geo=read(out/'courses.json');center=np.array(geo['center'])
    angle=np.linspace(0,np.pi/2,150);radius=geo['radius_m'];width=geo['width_m']
    fig,ax=plt.subplots(figsize=(9,9))
    for r in [radius-width/2,radius,radius+width/2]:
        points=center+r*np.stack([np.cos(angle),np.sin(angle)],axis=1)
        ax.plot(points[:,0],points[:,1],color='#888',linestyle='--' if r==radius else '-',linewidth=1.2)
    selected=['350z','redline-911','ref-a13','ref-e09','ref-e03','ref-j12'];names={c['id']:c['name'] for c in roster}
    for car in selected:
        row=next(r for r in data['runs'] if r['car']==car and r['case']=='curve-forward-fast' and r['repeat']==1)
        path=ROOT/row['run']/'physics.jsonl'
        points=np.array([json.loads(line)['position'] for i,line in enumerate(path.open()) if i%12==0])
        ax.plot(points[:,0],points[:,2],label=names[car],alpha=.85)
    ax.set(aspect='equal',xlabel='World X (m)',ylabel='World Z (m)',title='Recorded paths through the new curve — shared 14 m/s target')
    ax.grid(alpha=.2);ax.legend(fontsize=9,loc='lower left');fig.tight_layout()
    fig.savefig(out/'curve-paths.png',dpi=170);plt.close(fig)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--benchmarks-only',action='store_true');args=parser.parse_args()
    roster=read(ROOT/'assets/cars/roster.json');benchmarks(args.output,roster)
    if not args.benchmarks_only:handling(args.output,roster);curve_paths(args.output,roster)


if __name__=='__main__':main()
