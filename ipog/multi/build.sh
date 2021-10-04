#!/usr/bin/env zsh

PGO_DATA=/tmp/pgo-mc-ipog-data/
PGO_DATA_RESULT=./mc-ipog-merged.profdata

PROF_DIR=./target/prof/
PROF_BIN=${PROF_DIR}release/examples/ipog
PROF_TEMP=${PROF_DIR}ipog.bak
PROF_COMMAND=$(echo ~/.rustup/toolchains/stable-*/lib/rustlib/*/bin/llvm-profdata | tail -n1)

OPTIM_DIR=./target/optimised/
OPTIM_BIN=${OPTIM_DIR}release/examples/ipog
OUTPUT=./ipog_optimised

set -x

rm -rf ${PGO_DATA} ${OPTIM_DIR} || exit 1
#rm -rf ${OPTIM_DIR} || exit 1

if [ -f ${PROF_BIN} ]; then
  cp ${PROF_BIN} ${PROF_TEMP} || exit 1
fi;

RUSTFLAGS="-Cprofile-generate=${PGO_DATA}" cargo build --color=always --example ipog --release --target-dir=${PROF_DIR} || exit 1

[ -f ${PROF_TEMP} ] && [ -f ${PGO_DATA_RESULT} ] && cmp --silent ${PROF_BIN} ${PROF_TEMP} && exit 0

for s in 2 3 4; do
  ${PROF_DIR}release/examples/ipog ../sut_parser/tests/benchmarks/STF11/Healthcare4.cocoa -s${s} || exit 1
done;

${PROF_DIR}release/examples/ipog ../sut_parser/tests/benchmarks/STF11/Insurance.cocoa -s5 || exit 1
${PROF_DIR}release/examples/ipog ../sut_parser/tests/benchmarks/STF11/Storage5.cocoa -s5 || exit 1

${PROF_DIR}release/examples/ipog ../sut_parser/tests/benchmarks/STF11/Healthcare3.cocoa -s6 || exit 1
${PROF_DIR}release/examples/ipog ../sut_parser/tests/benchmarks/STF11/ProcessorComm2.cocoa -s6 || exit 1
${PROF_DIR}release/examples/ipog ../sut_parser/tests/benchmarks/STF11/Services.cocoa -s6 || exit 1

${PROF_DIR}release/examples/ipog ../sut_parser/tests/benchmarks/STF11/NetworkMgmt.cocoa -s7 || exit 1
${PROF_DIR}release/examples/ipog ../sut_parser/tests/benchmarks/STF11/Storage3.cocoa -s7 || exit 1

for s in 8 9 10 11 12; do
  ${PROF_DIR}release/examples/ipog ../sut_parser/tests/benchmarks/STF11/ProcessorComm1.cocoa -s${s} || exit 1
done;

${PROF_COMMAND} merge -o ${PGO_DATA_RESULT} ${PGO_DATA} || exit 1

cp -r ${PROF_DIR} ${OPTIM_DIR} || exit 1

RUSTFLAGS="-Cprofile-use=${PGO_DATA_RESULT} -Cllvm-args=-pgo-warn-missing-function" cargo build --color=always --example ipog --release --target-dir=${OPTIM_DIR} || exit 1

[ -f ${OUTPUT} ] || ln -s ${OPTIM_BIN} ${OUTPUT}
