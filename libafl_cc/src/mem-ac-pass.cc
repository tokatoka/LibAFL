/*
   LibAFL - Ctx LLVM pass
   --------------------------------------------------

   Written by Dongjia Zhang <toka@aflplus.plus>

   Copyright 2022-2023 AFLplusplus Project. All rights reserved.

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at:

     http://www.apache.org/licenses/LICENSE-2.0

*/

#include <stdio.h>
#include <stdlib.h>
#include "common-llvm.h"
#ifndef _WIN32
  #include <unistd.h>
  #include <sys/time.h>
#else
  #include <io.h>
#endif
#include <string.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <ctype.h>

#include <list>
#include <string>
#include <fstream>
#include <set>

#include "llvm/Config/llvm-config.h"
#include "llvm/ADT/Statistic.h"
#include "llvm/IR/IRBuilder.h"

#include "llvm/IR/BasicBlock.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/DebugInfo.h"
#include "llvm/IR/CFG.h"
#include "llvm/IR/Verifier.h"
#include "llvm/Support/Debug.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Transforms/Utils/BasicBlockUtils.h"
#include "llvm/Analysis/LoopInfo.h"
#include "llvm/Analysis/ValueTracking.h"
#include "llvm/Pass.h"
#include "llvm/IR/Constants.h"

#include <iostream>

using namespace llvm;

#define MAP_SIZE LIBAFL_EDGES_MAP_SIZE

namespace {

#if USE_NEW_PM
class CtxPass : public PassInfoMixin<CtxPass> {
 public:
  CtxPass() {
#else
class CtxPass : public ModulePass {
 public:
  static char ID;

  CtxPass() : ModulePass(ID) {
#endif
  }

#if USE_NEW_PM
  PreservedAnalyses run(Module &M, ModuleAnalysisManager &MAM);
#else
  bool runOnModule(Module &M) override;
#endif

 protected:
  uint32_t map_size = MAP_SIZE;

 private:
  bool isLLVMIntrinsicFn(StringRef &n) {
    // Not interested in these LLVM's functions
    if (n.starts_with("llvm.")) {
      return true;
    } else {
      return false;
    }
  }
};

}  // namespace

#if USE_NEW_PM
extern "C" ::llvm::PassPluginLibraryInfo LLVM_ATTRIBUTE_WEAK
llvmGetPassPluginInfo() {
  return {LLVM_PLUGIN_API_VERSION, "CtxPass", "v0.1",
          /* lambda to insert our pass into the pass pipeline. */
          [](PassBuilder &PB) {

  #if LLVM_VERSION_MAJOR <= 13
            using OptimizationLevel = typename PassBuilder::OptimizationLevel;
  #endif
            PB.registerOptimizerLastEPCallback(
                [](ModulePassManager &MPM, OptimizationLevel OL) {
                  MPM.addPass(CtxPass());
                });
          }};
}
#else
char CtxPass::ID = 0;
#endif

#if USE_NEW_PM
PreservedAnalyses CtxPass::run(Module &M, ModuleAnalysisManager &MAM) {
#else
bool CtxPass::runOnModule(Module &M) {

#endif
  LLVMContext   &C = M.getContext();
  auto           moduleName = M.getName();
  Type          *VoidTy = Type::getVoidTy(C);
  IntegerType   *Int64Ty = IntegerType::getInt64Ty(C);
  FunctionCallee hookRead;
  FunctionCallee hookWrite;

  uint32_t rand_seed;

  hookRead = M.getOrInsertFunction("__libafl_hook_read", VoidTy, Int64Ty);
  hookWrite = M.getOrInsertFunction("__libafl_hook_read", VoidTy, Int64Ty);

  for (auto &F : M) {
    int has_calls = 0;

    if (isIgnoreFunction(&F)) { continue; }
    if (F.size() < 1) { continue; }
    for (auto &BB : F) {
      for (auto &IN : BB) {
        LoadInst *loadInst = nullptr;
        StoreInst *storeInst = nullptr;
        // iterate over every store to instrument
        if ((loadInst = dyn_cast<LoadInst>(&IN))) {
          Value *mem = loadInst -> getPointerOperand();
          std::vector<Value *> args;
          args.push_back(mem);
          IRBuilder<> IRB(loadInst->getParent());
          IRB.SetInsertPoint(loadInst);

          IRB.CreateCall(hookRead, args);
        }
        else if ((storeInst = dyn_cast<StoreInst>(&IN))) {
          Value *mem = storeInst -> getPointerOperand();
          std::vector<Value *> args;
          args.push_back(mem);
          IRBuilder<> IRB(storeInst->getParent());
          IRB.SetInsertPoint(storeInst);

          IRB.CreateCall(hookWrite, args);
        }
        // else do nothing
      }
    }
  }

#if USE_NEW_PM
  auto PA = PreservedAnalyses::all();
  return PA;
#else
  return true;
#endif
}

#if USE_NEW_PM

#else
static void registerCtxPass(const PassManagerBuilder &,
                            legacy::PassManagerBase &PM) {
  PM.add(new CtxPass());
}

static RegisterPass<CtxPass> X("ctx", "ctx instrumentation pass", false, false);

static RegisterStandardPasses RegisterCtxPass(
    PassManagerBuilder::EP_OptimizerLast, registerCtxPass);

static RegisterStandardPasses RegisterCtxPass0(
    PassManagerBuilder::EP_EnabledOnOptLevel0, registerCtxPass);
#endif
