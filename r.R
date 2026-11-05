
library(ggdist);library(jsonlite)
s<-fromJSON("samples.json");out<-list()
for(n in names(s)){x<-as.numeric(s[[n]]);w<-c(0.5,0.9)
 o<-list()
 for(f in c("median_qi","mean_qi","mode_qi")){r<-suppressWarnings(get(f)(x,.width=w));o[[f]]<-list(y=r$y,lo=r$ymin,hi=r$ymax)}
 o$bw<-tryCatch(bw.SJ(x,method="dpi"),error=function(e)NA)
 o$mode_bounded<-tryCatch({d<-density_bounded(x,n=501);d$x[which.max(d$y)]},error=function(e)NA)
 out[[n]]<-o}
write(toJSON(out,auto_unbox=TRUE,digits=12),"r.json")
