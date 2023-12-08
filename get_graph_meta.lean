import Lean
import Lean.Data.Json.FromToJson
open Lean Elab Term


def getExpr (x : MetaM Syntax) : TermElabM Expr := do
  let synt ← x
  elabTerm synt none

def getConstantBody (n : Name) : MetaM (Option Expr) := do
  let constInfo ← getConstInfo n
  let constValue := constInfo.value?
  return constValue


def getAllConstsFromConst (n : Name) := do
  let body ← getConstantBody n
  return (match body with
  | some body => body.getUsedConstants
  | none => {})

structure BFSState :=
  (g : HashMap Name (List Name))
  (outerLayer : List Name)

def getUsedConstantGraph (s : MetaM Syntax) : TermElabM (List (Name × List Name)) := do
  let expr ← getExpr s
  let name := expr.constName!

  -- make bfs from the specified root node

  -- the goal is to construct a hashmap where the key is the name of the const, and the entry is a list of names of other consts

  -- we keep a list of const names representing the outer layer of the bfs

  -- in each iteration we for each const in the outer layer find its references and that way construct the nodes that will be added to the graph

  -- then we extract the outer layer from the new nodes by looking at the children and checking whether they are already in the graph


  let state ← (List.range 10).foldlM (fun (state : BFSState) (i : Nat) => do
    let g := state.g
    let outerLayer := state.outerLayer

    let newNodes ← outerLayer.mapM fun name => do
      let consts ← getAllConstsFromConst name
      return (name, consts)

    let g := newNodes.foldl (fun m p => m.insert p.fst p.snd.toList) g
    let newOuterLayer := newNodes.foldl (fun (set : HashSet Name) (node : Name × Array Name) =>
      let set := set.insertMany node.snd;
      set) mkHashSet
    let newOuterLayer := newOuterLayer.toList.filter (fun n => !(g.contains n))

    return BFSState.mk g newOuterLayer
  )
    (BFSState.mk mkHashMap [name])




  return state.g.toList


#synth ToJson (List (Name × List Name))

def writeJsonToFile (filePath : String) (json : Json) : IO Unit := do
  let jsonString := toString json
  IO.FS.withFile filePath IO.FS.Mode.write fun handle => do
    handle.putStr jsonString

-- Convert a Name to a String
def nameToString (n : Name) : String :=
  toString n

-- Convert a Name and List Name pair to JSON
def pairToJson (pair : Name × List Name) : Json :=
  let nameStr := nameToString pair.fst
  let nameListStr := pair.snd.map nameToString
  Json.mkObj [("name", Json.str nameStr), ("references", Json.arr (nameListStr.map Json.str).toArray)]

-- Serialize a List (Name, List Name) to JSON
def serializeList (l : List (Name × List Name)) : Json :=
  Json.arr (l.map pairToJson).toArray


def serializeAndWriteToFile := do
  let g ← getUsedConstantGraph `(Nat.add_comm)
  let js := serializeList g
  let kk ← writeJsonToFile "add_comm.json" js

#eval serializeAndWriteToFile
