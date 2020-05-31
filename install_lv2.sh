#!/bin/bash

cargo build --release || exit 1

[ ! -d $HOME/.lv2 ] && mkdir $HOME/.lv2
[ ! -d $HOME/.lv2/sonarigo.lv2 ] && mkdir $HOME/.lv2/sonarigo.lv2

cp target/release/libsonarigo_lv2.so $HOME/.lv2/sonarigo.lv2/ || exit
cp sonarigo-lv2/lv2/*ttl $HOME/.lv2/sonarigo.lv2/ || exit

echo
echo sonarigo.lv2 successfully installed
