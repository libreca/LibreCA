#!/usr/bin/env zsh

EXAMPLE_NAME=ipog_bh_bv
PGO_DATA=/tmp/pgo-ipog-data/
PGO_DATA_RESULT=$(realpath ./ipog-merged.profdata)

PROF_DIR=./target/prof/
PROF_BIN=${PROF_DIR}release/examples/${EXAMPLE_NAME}
PROF_TEMP=${PROF_DIR}ipog.bak
PROF_COMMAND=$(echo ~/.rustup/toolchains/stable-*/lib/rustlib/*/bin/llvm-profdata | tail -n1)

NORMAL_DIR=./target/normal/
NORMAL_BIN=${NORMAL_DIR}release/examples/${EXAMPLE_NAME}
NORMAL=./ipog_normal

OPTIM_DIR=./target/optimised/
OPTIM_BIN=${OPTIM_DIR}release/examples/${EXAMPLE_NAME}
OUTPUT=./ipog_optimised

FLAGS="+adx,+aes,+avx,+avx2,+bmi,+bmi2,+clflushopt,+cmov,+cx16,+cx8,+ermsb,+f16c,+fma,+fsgsbase,+fxsr,+invpcid,+mmx,+movbe,+mpx,+nopl,+pclmul,+popcnt,+rdrnd,+rdseed,+rtm,+sse,+sse2,+sse4.1,+sse4.2,+ssse3,+xsave,+xsavec,+xsaveopt,+xsaves"

set -x

rm -rf ${PGO_DATA} ${OPTIM_DIR} || exit 1
#rm -rf ${OPTIM_DIR} || exit 1

if [ -f ${PROF_BIN} ]; then
  cp ${PROF_BIN} ${PROF_TEMP} || exit 1
fi;

RUSTFLAGS="-Cprofile-generate=${PGO_DATA} -C target-cpu=native -C target-feature=${FLAGS}" cargo build --color=always --example ${EXAMPLE_NAME} --release --target-dir=${PROF_DIR} || exit 1

if [ -f ${PROF_TEMP} ] && [ -f "${PGO_DATA_RESULT}" ] && cmp --silent ${PROF_BIN} ${PROF_TEMP}; then
  echo "No change to profiler; skipping benchmarks"
else
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Storage5.cocoa       -ns2  || exit 1 # Should take: 0.001
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Insurance.cocoa      -ns2  || exit 1 # Should take: 0.001
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Storage5.cocoa       -ns3  || exit 1 # Should take: 0.021
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Insurance.cocoa      -ns3  || exit 1 # Should take: 0.018
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Storage5.cocoa       -ns4  || exit 1 # Should take: 0.603
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Healthcare4.cocoa    -ns4  || exit 1 # Should take: 0.483
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Storage5.cocoa       -ns5  || exit 1 # Should take: 18.575
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Healthcare4.cocoa    -ns5  || exit 1 # Should take: 13.234
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Healthcare3.cocoa    -ns6  || exit 1 # Should take: 48.365
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/coveringcerts.cocoa  -ns6  || exit 1 # Should take: 36.945
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Storage3.cocoa       -ns7  || exit 1 # Should take: 3.936
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/ProcessorComm1.cocoa -ns7  || exit 1 # Should take: 2.498
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Storage3.cocoa       -ns8  || exit 1 # Should take: 27.254
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/ProcessorComm1.cocoa -ns8  || exit 1 # Should take: 12.449
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Healthcare2.cocoa    -ns9  || exit 1 # Should take: 0.307
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Banking2.cocoa       -ns9  || exit 1 # Should take: 0.273
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Banking2.cocoa       -ns10 || exit 1 # Should take: 0.393
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Healthcare2.cocoa    -ns10 || exit 1 # Should take: 0.251
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Banking2.cocoa       -ns11 || exit 1 # Should take: 0.449
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Healthcare2.cocoa    -ns11 || exit 1 # Should take: 0.016
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Services.cocoa       -ns12 || exit 1 # Should take: 38.269
  ${PROF_DIR}release/examples/${EXAMPLE_NAME} ../sut_parser/tests/benchmarks/STF11/Banking2.cocoa       -ns12 || exit 1 # Should take: 0.603

  ${PROF_COMMAND} merge -o ${PGO_DATA_RESULT} ${PGO_DATA} || exit 1

  cp -r ${PROF_DIR} ${OPTIM_DIR} || exit 1
fi

RUSTFLAGS="-Cprofile-use=${PGO_DATA_RESULT} -Cllvm-args=-pgo-warn-missing-function -C target-cpu=native -C target-feature=${FLAGS}" cargo build --color=always --example ${EXAMPLE_NAME} --release --target-dir=${OPTIM_DIR} || exit 1
RUSTFLAGS="-C target-cpu=native -C target-feature=${FLAGS}" cargo build --color=always --example ${EXAMPLE_NAME} --release --target-dir=${NORMAL_DIR} || exit 1

[ -f ${OUTPUT} ] || ln -s ${OPTIM_BIN} ${OUTPUT}
[ -f ${NORMAL} ] || ln -s ${NORMAL_BIN} ${NORMAL}

${OUTPUT} ../sut_parser/tests/benchmarks/STF11/Storage3.cocoa -ns8
${NORMAL} ../sut_parser/tests/benchmarks/STF11/Storage3.cocoa -ns8
