"""Create inspectable scientific figures for the final common vehicle matrix."""
import argparse
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt


def read(path):
    return json.loads(path.read_text())


def risk_plot(out, evaluation, roster, profiles):
    cars={r['id']:r for r in roster};families={(r['car'],r['family']):r for r in evaluation['family_metrics']}
    x=[];y=[];colors=[];labels=[]
    for car in roster:
        track=sum(abs(car['wheels'][i+1][0]-car['wheels'][i][0]) for i in [0,2])/2
        row=families[car['id'],'trail-brake']
        x.append(profiles[car['id']]['cg_height_m']/track);y.append(row['maximum_tilt_deg'])
        colors.append('#c35b41' if row['failed_pairs'] else '#277d8b');labels.append(car['id'])
    fig,ax=plt.subplots(figsize=(11,6),constrained_layout=True)
    ax.scatter(x,y,c=colors,s=24,alpha=.65)
    selected=sorted(range(len(roster)),key=lambda i:y[i],reverse=True)[:2]
    for i in selected:
        ax.annotate(cars[labels[i]]['name'],(x[i],y[i]),xytext=(-12,18),ha='right',textcoords='offset points',fontsize=8)
    ax.axhline(80,color='#777',ls='--',lw=1,label='80° stress flag')
    ax.set_xlabel('Declared CG height / mean wheel track (many CG values estimated)')
    ax.set_ylabel('Maximum tilt in 500 common trail-braking cases (degrees)')
    ax.set_title('300 source editions: rollover risk under common abrupt inputs')
    ax.set_ylim(-5,198);ax.grid(alpha=.15);ax.legend(loc='upper left')
    fig.savefig(out/'geometry-and-tilt.png',dpi=160);plt.close(fig)


def flagged_ranking(cars):
    ranked=[]
    for car in cars:
        count=sum(row['failed_pairs'] for row in car['families'])
        if count:ranked.append((count,car['car']))
    return sorted(ranked,reverse=True)[:15]


def summary_plot(out, evaluation, roster):
    names={r['id']:r['name'] for r in roster};cars=evaluation['car_results']
    before=sum(c['original_slope_before']['failure_reasons'].get('unexpected-reset',0) for c in cars)
    after=sum(r['failure_reasons'].get('unexpected-reset',0) for r in evaluation['family_metrics'] if r['family']=='slope')
    risk=flagged_ranking(cars)
    speed=sorted([(c['selected_common_cases']['49']['final_speed_mps']*3.6,c['car']) for c in cars])
    chosen=speed[:5]+speed[-5:]
    fig,axes=plt.subplots(1,3,figsize=(17,6),constrained_layout=True)
    bars=axes[0].bar(['Original slope','Raised slope'],[before,after],color=['#9ba8b8','#167b88'])
    axes[0].bar_label(bars,padding=3);axes[0].set_ylim(0,max(1,before)*1.18)
    axes[0].set_title('Supported-ground fall resets');axes[0].set_ylabel('Flagged car/case pairs (two repeats each)')
    axes[1].barh(range(len(risk)),[r[0] for r in risk],color='#c35b41')
    axes[1].set_yticks(range(len(risk)),[names[r[1]][:32] for r in risk]);axes[1].invert_yaxis()
    axes[1].set_xlabel('Flagged pairs after track correction');axes[1].set_title('Remaining physical stress findings')
    axes[2].barh(range(len(chosen)),[r[0] for r in chosen],color=['#687e91' if r[1].startswith('exp-') else '#167b88' for r in chosen])
    axes[2].set_yticks(range(len(chosen)),[names[r[1]][:27]+' ['+r[1]+']' for r in chosen]);axes[2].invert_yaxis()
    axes[2].set_xlabel('km/h after 20 s, full-throttle case 49');axes[2].set_title('Common launch response: slowest / fastest')
    for ax in axes:ax.grid(axis='x',alpha=.15);ax.set_axisbelow(True)
    fig.suptitle('300 cars × 5,024 common scenarios × two repeats; proxies are not real-world validation',fontsize=13)
    fig.savefig(out/'fleet-summary.png',dpi=160);plt.close(fig)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True);parser.add_argument('--source',type=Path,required=True)
    args=parser.parse_args();evaluation=read(args.output/'evaluation.json')
    roster=read(args.source/'assets/cars/roster.json');profiles={r['id']:r['parameters'] for r in read(args.source/'assets/cars/physics.json')['cars']}
    risk_plot(args.output,evaluation,roster,profiles);summary_plot(args.output,evaluation,roster)


if __name__=='__main__':main()
