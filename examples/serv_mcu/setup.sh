#!/bin/bash
pip -m venv venv
source venv/bin/activate
pip install bronzebeard
git clone https://github.com/olofk/serv.git /tmp/serv
cp -r /tmp/serv/rtl examples/serv_mcu/serv
