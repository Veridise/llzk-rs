module attributes { veridise.lang = "llzk" } {
struct.def @empty<[@T]> {
  function.def @compute() -> !struct.type<@empty<[@T]>> attributes {function.allow_witness} {
    %self = struct.new : <@empty<[@T]>>
    function.return %self : !struct.type<@empty<[@T]>>
  }
  function.def @constrain(%arg0: !struct.type<@empty<[@T]>>) attributes {function.allow_constraint} {
    function.return
  }
}
}
