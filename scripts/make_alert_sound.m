fS=44100;
f1=440;
a1=0.7;
f2=1500;
a2=0.3;
t_on=1;
t_edge=1/250;
t_off=0.5;
count=5;
t_tot=t_on + t_off;
m=ceil(t_tot*fS);
ts=(0:(m-1))'/fS;
eps=0.01;

function [y]=smooth_edge(xs,x0,dx,epsilon)
  y=(tanh((xs-x0)*atanh(1-epsilon)/dx)+1)/2;
end

env=smooth_edge(ts,t_edge,t_edge,eps).*...
    (1-smooth_edge(ts,t_on - t_edge,t_edge,eps));
tone=0.5*(a1*cos(2*pi*f1*ts) + a2*cos(2*pi*f2*ts));
#+cos(2*pi*f2*ts));
u=env.*tone;
v=repmat(u,[count,1]);
audiowrite("alert.wav",v,fS);
