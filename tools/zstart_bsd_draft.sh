#!/bin/sh

zProjPath=$1  # rust project root path
zPgDataPath=/var/db/postgres/data10
zPgBinPath=/usr/local/bin

if [ "" == ${zProjPath} ]; then
    printf "\033[31;01mprojPath missing !!\033[00m\n"
    exit 1
fi

cd ${zProjPath}
if [ 0 -ne $? ]; then
    printf "\033[31;01mprojPath invalid: ${zProjPath} !!\033[00m\n"
    exit 1
fi

if [ 0 == `\ls ${HOME}/.pgpass | wc -l` ]; then
    echo "# hostname:port:database:username:password" > ${HOME}/.pgpass
fi
chmod 0600 ${HOME}/.pgpass

# build postgreSQL
# ===> install from ports

# PG: 留空表示仅接受本地 UNIX 域套接字连接
sed -i.bak '/#*listen_addresses =/d' ${zPgDataPath}/postgresql.conf
echo "listen_addresses = ''" >> ${zPgDataPath}/postgresql.conf

# PG: 以源码根路径作为 UNIX 域套接字存放路径
sed -i.bak '/#*unix_socket_directories =/d' ${zPgDataPath}/postgresql.conf
echo "unix_socket_directories = '${HOME}'" >> ${zPgDataPath}/postgresql.conf

# PG: UNIX 域套接字权限
sed -i.bak '/#*unix_socket_permissions =/d' ${zPgDataPath}/postgresql.conf
echo "unix_socket_permissions = 0700" >> ${zPgDataPath}/postgresql.conf

${zPgBinPath}/pg_ctl -D ${zPgDataPath} -l ${zPgDataPath}logfile stop
${zPgBinPath}/pg_ctl -D ${zPgDataPath} -l ${zPgDataPath}logfile start
${zPgBinPath}/createdb -O `whoami` svdp

# 借助 nohup 进入守护进程模式
cd ${zProjPath}
killall svdp
nohup cargo run --release &
if [ 0 -ne $? ]; then
    printf "\033[31;01msvdp start failed !!\033[00m\n"
    exit 1
fi
