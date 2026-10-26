"""Build the frozen GT500 comparison and figure; run from any directory."""
import csv
import json
from pathlib import Path

OUT=Path(__file__).resolve().parent
ROOT=OUT.parents[2]
ART=ROOT/'artifacts/vehicle-reference/gt500-midrange'
OLD=ROOT/'artifacts/vehicle-reference/fleet-40-calibration'
def read(p):return json.loads(p.read_text())
def indexed(p):return {r['car']:r for r in read(p)}
def checkpoints(r):return {int(x['mph']):x['seconds'] for x in r['checkpoints']}
def force_utilization(trace):
    ratios=[r['applied_n']/r['demand_n'] for r in trace
            if 26.8224<=r['speed_mps']<=44.704 and r['demand_n']>0]
    return {'minimum':min(ratios),'maximum':max(ratios)}

def build():
    before=indexed(OLD/'after-benchmark-r1.json');after=indexed(ART/'after-benchmark-r1.json')
    bs={(r['car'],r['diameter_ft']):r for r in read(OLD/'after-skidpad-r1.json')}
    ass={(r['car'],r['diameter_ft']):r for r in read(ART/'after-skidpad-r1.json')}
    rows=[]
    for car in before:
        r={'car':car}
        for key,val in before[car].items():
            if key=='car':continue
            r['before_'+key]=val;r['after_'+key]=after[car][key]
        for d in [200,300]:
            r[f'before_circle_{d}ft_g']=bs[car,d]['peak_sustained_g']
            r[f'after_circle_{d}ft_g']=ass[car,d]['peak_sustained_g']
        rows.append(r)
    with (OUT/'all-40-fixtures.csv').open('w') as f:
        w=csv.DictWriter(f,list(rows[0]));w.writeheader();w.writerows(rows)
    first=read(ART/'baseline-correct-distance.json');last=read(ART/'after-trace-r1.json')
    b=checkpoints(first);a=checkpoints(last)
    data={'car':'redline-gt500','selected_parameter':{'name':'driveline_efficiency','before':.88,'after':.82,'status':'estimate'},
          'source':read(ART/'diagnostic-method.json'),'speed_checkpoints':[{'mph':m,'reference_s':ref,'before_s':b[m],'after_s':a[m]} for m,ref in [(30,1.8),(40,2.5),(50,3.3),(60,4.3),(70,5.4),(80,6.7),(90,8.1),(100,9.7)]],
          'sixty_to_hundred_s':{'reference':5.4,'before':b[100]-b[60],'after':a[100]-a[60]},
          'quarter_mile':{'reference':{'seconds':12.6,'mph':114.2},'before':first['quarter_mile'],'after':last['quarter_mile'],'qualification':'Simulation uses no rollout and instantaneous crossing speed; publication timing-trap details and test load unknown.'},
          'handling':[{'diameter_ft':d,'before':bs['redline-gt500',d],'after':ass['redline-gt500',d]} for d in [200,300]],
          'unchanged_other_car_benchmark_rows':sum(before[c]==after[c] for c in before if c!='redline-gt500'),
          'unchanged_other_car_circle_rows':sum(bs[k]==ass[k] for k in bs if k[0]!='redline-gt500'),
          'baseline_force_utilization_60_100':force_utilization(first['trace'])}
    (OUT/'comparison.json').write_text(json.dumps(data,indent=2)+'\n')
    return data

def plot(data):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig,axes=plt.subplots(1,2,figsize=(13,5.4))
    colors={'before':'#b96928','after':'#176e91','reference':'#273a45'}
    points=data['speed_checkpoints'];mph=[r['mph'] for r in points]
    for label in ['before','after','reference']:
        axes[0].plot(mph,[r[label+'_s'] for r in points],marker='o',color=colors[label],label=label.title())
    axes[0].set(xlabel='Speed checkpoint (mph)',ylabel='Elapsed time from rest (s)',title='GT500: same production physics and inputs')
    axes[0].legend(frameon=False);axes[0].grid(alpha=.2)
    intervals=[('60–80',60,80),('80–100',80,100)]
    points={r['mph']:r for r in points}
    for j,label in enumerate(['before','after','reference']):
        vals=[points[hi][label+'_s']-points[lo][label+'_s'] for _,lo,hi in intervals]
        xs=[i+(j-1)*.24 for i in range(2)]
        bars=axes[1].bar(xs,vals,width=.22,color=colors[label],label=label.title())
        axes[1].bar_label(bars,fmt='%.2f',padding=3,fontsize=9)
    axes[1].set_xticks([0,1],[v[0]+' mph' for v in intervals]);axes[1].set(ylabel='Interval duration (s)',title='Midrange intervals: residual differences stay visible',ylim=(0,3.8))
    axes[1].grid(axis='y',alpha=.2);axes[1].set_axisbelow(True)
    fig.suptitle('GT500 effective power delivery: 0.88 → 0.82',x=.04,ha='left',fontsize=15)
    fig.text(.04,.02,'Reference: July 2008 MotorTrend coupe test. Simulation: 120 Hz, nominal curb mass, no rollout.\nEffective delivery is an estimate; tire limits, brakes, suspension and rated output are unchanged. Calibration is not independent validation.',fontsize=9)
    fig.tight_layout(rect=[0,.1,1,.92]);fig.savefig(OUT/'acceleration-comparison.png',dpi=160);plt.close(fig)



def plot_scenarios():
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    before=read(ROOT/'docs/verification/fleet-40-calibration/results.json')
    after=read(OUT/'results.json')
    phases=[{r['case']:r for r in phase['aggregate'] if r['car']=='redline-gt500'} for phase in [before,after]]
    cases=after['study']['cases'];fig,axes=plt.subplots(1,3,figsize=(15,11),sharey=True)
    for ax,key,title in zip(axes,['peak_speed_mps','peak_tilt_deg','four_contact_pct'],['Peak speed (m/s)','Peak chassis tilt (degrees)','Four wheel-ray contacts (%)']):
        for j,(phase,color,label) in enumerate(zip(phases,['#b96928','#176e91'],['Before','After'])):
            ys=[i+(j-.5)*.34 for i in range(len(cases))];values=[phase[c][key] for c in cases]
            if key=='four_contact_pct':ax.scatter(values,ys,color=color,marker=['o','s'][j],s=22,label=label)
            else:ax.barh(ys,values,height=.32,color=color,label=label)
        ax.set_title(title);ax.grid(axis='x',alpha=.2);ax.set_axisbelow(True)
    axes[0].set_yticks(range(len(cases)),cases,fontsize=8);axes[0].invert_yaxis();axes[0].legend(frameon=False)
    axes[2].set_xlim(90,100.5)
    fig.suptitle('GT500: identical 24 city cases, two repetitions each',x=.03,ha='left',fontsize=15)
    fig.text(.03,.015,'All case inputs, geometry, thresholds and seeds are unchanged. Bars summarize repeat means.\nTilt includes map impacts; four ray contacts do not guarantee four loaded tires. Passing case assertions is separate from contact-free travel.',fontsize=9)
    fig.tight_layout(rect=[0,.045,1,.95]);fig.savefig(OUT/'scenario-comparison.png',dpi=150);plt.close(fig)

if __name__=='__main__':
    plot(build())
    if (OUT/'results.json').exists():plot_scenarios()
