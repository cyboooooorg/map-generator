import run
Ns=[10,20,40,80,160,320,640]
ref=2560
worst={}
for cand in ['A','B','C','D1','D2']:
    for body in ['Earth','Mars(0.21)','Mercury','Moon','Ganymede','5ME_50%H2O','10ME']:
        r0=run.run(cand,body,N=ref)
        row=[]
        for N in Ns:
            r=run.run(cand,body,N=N)
            row.append((N,abs(r['R']/r0['R']-1),abs(r['C']/r0['C']-1)))
        need=next((N for N,a,b in row if all(x[1]<1e-4 and x[2]<1e-4 for x in row if x[0]>=N)),None)
        worst[(cand,body)]=need
        print(cand,body,'ref R',round(r0['R']/1e3,2),'N for 1e-4:',need,' errs@40:',"%.1e %.1e"%row[2][1:],' @160:',"%.1e %.1e"%row[4][1:],flush=True)
print('max N needed', max(v for v in worst.values() if v))
