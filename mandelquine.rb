







                                                                          eval(s=%w(
                                                                       k=0;n
                                                                       =(k+1)
                                                                       %120;u
                                                                   = 0 . 01 792
                                                           33  ;f=u**(n/120.0);w=11
                                                            0;h=48;m=100+10*n;pr=(-1.7 5-u
                                                         /4)/(1-u);cr=pr+(-0.65-pr)*f;sc=
                                                        3.05*f/w;v=(0...w*h).map{|j|a=0;4.
                                                       times{|q|c=Complex(cr+(j%w+q%2*0.5+0
                                                      .25-w*0.5)*sc,(h*0.5-j/w-q/2*0.5-0.25)
                                     *    2          *sc);z=0;i=(1..m).find{z=z*z+c;z.abs2>6
                                     4};a+=i&&i-    Math.log2(Math.log(z.abs))||m*2};a};o=(0
                                   ...w*h).sort_by  {|j|[-v[j],j]};t=s.split(35.chr)[0].sub(
                                  /-?[0-9]+/){n};l= t.size+15;ic=v.count{|x|x>=8*m};p=[l,ic
                             ].max;g=a=b=0;lay=->(q){g=[nil]*(w*h);o[0,q].each{|j|g[j]=1}
                             ;a=g.index(1);a=[a,a/w*w+w-10].min;10.times{|d|g[a+d]=1};b=g
                                  .rindex(1);b=[b,b /w*w+4].max;5.times{|d|g[b-d]=1};g.coun
                                   t(1)};c=lay[p];  c=lay[p-=1]while(p>ic&&c>l);o.each{|j|c<
                                     l&&!g[j]&&j    >a&&j<b&&(g[j]=1;c+=1)};fx='';fq=fd=0;c>
                                     l    &          &((t*c).each_char{|ch|dd=81-2*ch.ord;dd
                                                      .abs>1&&dd=0;fq+fd+1+dd>c-l-1&&break;f
                                                       q+=1;fd+=dd};fx=35.chr+(t*c)[0,fq]+5
                                                        9.chr*(c-l-1-fq-fd)+41.chr*fd);e=(
                                                         'eval(s=%w('+t+fx+')*"")').chars
                                                            ;puts((0...h).map{|y|(0... w).
                                                           ma  p{|x|g[y*w+x]&&e.shi
                                                                   f t | |3 2.c
                                                                       hr}.jo
                                                                       in.rst
                                                                       rip})
                                                                      )*"")








