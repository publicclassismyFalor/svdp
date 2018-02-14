#!/usr/bin/env bash
export zPgPath=${HOME}/.____PostgreSQL

zProjPath=$1  # rust project root path
zSystemd=$2
zPostgresVersion="10.2"

if [[ "" == ${zProjPath} ]]; then
    printf "\033[31;01mprojPath missing !!\033[00m\n"
    exit 1
fi

cd ${zProjPath}
if [[ 0 -ne $? ]]; then
    printf "\033[31;01mprojPath invalid: ${zProjPath} !!\033[00m\n"
    exit 1
fi

if [[ 0 == `\ls ${HOME}/.pgpass | wc -l` ]]; then
    echo "# hostname:port:database:username:password" > ${HOME}/.pgpass
fi
chmod 0600 ${HOME}/.pgpass

# build postgreSQL
if [[ 0 -eq `\ls -d ${zPgPath} | wc -l` ]]; then
    cd ${HOME}
    if [[ 0 -eq `\ls postgresql-${zPostgresVersion}.tar.bz2 | wc -l` ]]; then
        wget https://ftp.postgresql.org/pub/source/v${zPostgresVersion}/postgresql-${zPostgresVersion}.tar.bz2
    fi

    tar -xf postgresql-${zPostgresVersion}.tar.bz2
    if [[ 0 -ne $? ]]; then
        printf "\033[31;01mfailed: tar -xf postgresql-${zPostgresVersion}.tar.bz2 !!\033[00m\n"
        exit 1
    fi

    cd postgresql-${zPostgresVersion}
    if [[ "" == ${zSystemd} ]]; then
        ./configure --prefix=${zPgPath}
    else
        ./configure --prefix=${zPgPath} --with-systemd
    fi

    make -j `cat /proc/cpuinfo| grep processor | wc -l` && make install
fi

# start postgresql
zPgPath=${HOME}/.____PostgreSQL
zPgLibPath=${zPgPath}/lib
zPgBinPath=${zPgPath}/bin
zPgDataPath=${zPgPath}/data

# PG: 留空表示仅接受本地 UNIX 域套接字连接
sed -i '/#*listen_addresses =/d' ${zPgDataPath}/postgresql.conf
echo "listen_addresses = ''" >> ${zPgDataPath}/postgresql.conf

# PG: 以源码根路径作为 UNIX 域套接字存放路径
sed -i '/#*unix_socket_directories =/d' ${zPgDataPath}/postgresql.conf
echo "unix_socket_directories = '${HOME}'" >> ${zPgDataPath}/postgresql.conf

# PG: UNIX 域套接字权限
sed -i '/#*unix_socket_permissions =/d' ${zPgDataPath}/postgresql.conf
echo "unix_socket_permissions = 0700" >> ${zPgDataPath}/postgresql.conf

# PG: 最大连接数
sed -i '/#*max_connections =/d' ${zPgDataPath}/postgresql.conf
echo "max_connections = 1024" >> ${zPgDataPath}/postgresql.conf

# PG: 事务锁上限
sed -i '/#*max_locks_per_transaction =/d' ${zPgDataPath}/postgresql.conf
echo "max_locks_per_transaction = 512" >> ${zPgDataPath}/postgresql.conf

# PG: shared buffers siz，设置为总内存的 1/3
sed -i '/#*shared_buffers =/d' ${zPgDataPath}/postgresql.conf
echo "shared_buffers = $((`free -m | fgrep -i 'mem' | awk -F' ' '{print $2}'` / 3))MB" >> ${zPgDataPath}/postgresql.conf

# PG: max_wal_size，设置为总内存的 1/2
sed -i '/#*max_wal_size =/d' ${zPgDataPath}/postgresql.conf
echo "max_wal_size = $((`free -m | fgrep -i 'mem' | awk -F' ' '{print $2}'` / 2))MB" >> ${zPgDataPath}/postgresql.conf

# PG: work_mem，设置为 64MB
sed -i '/#*work_mem =/d' ${zPgDataPath}/postgresql.conf
echo "work_mem = 64MB" >> ${zPgDataPath}/postgresql.conf

# PG: max_stack_depth，设置为系统线程栈的大小 - 1M
sed -i '/#*max_stack_depth =/d' ${zPgDataPath}/postgresql.conf
echo "max_stack_depth = $((`ulimit -s` / 1024 - 1))MB" >> ${zPgDataPath}/postgresql.conf

${zPgBinPath}/pg_ctl -D ${zPgDataPath} initdb
${zPgBinPath}/pg_ctl stop -D ${zPgDataPath} -l ${zPgDataPath}/log
${zPgBinPath}/pg_ctl start -D ${zPgDataPath} -l ${zPgDataPath}/log
${zPgBinPath}/createdb -O `whoami` svdp

# 需要 root 权限，防止 postgresql 主进程被 linux OOM_killer 杀掉
# zPgPid=`head -1 ${zPgDataPath}/postmaster.pid`
# (echo -1000 > /proc/$pid/oom_score_adj; echo -17 > /proc/$pid/oom_adj) 2>${zPgDataPath}/log

# 借助 nohup 进入守护进程模式
cd ${zProjPath}
nohup cargo run --release &
if [[ 0 -ne $? ]]; then
    printf "\033[31;01msvdp start failed !!\033[00m\n"
    exit 1
fi
