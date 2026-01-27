module attributes { veridise.lang = "llzk" } {
  struct.def @empty<[]> {
    function.def @compute() -> !struct.type<@empty<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
      %self = struct.new : <@empty<[]>>
      function.return %self : !struct.type<@empty<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@empty<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
      function.return
    }
  }
}
