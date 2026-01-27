module attributes {veridise.lang = "llzk"} {
  struct.def @StructA<[]> {
    function.def @compute() -> !struct.type<@StructA<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
      %self = struct.new : <@StructA<[]>>
      function.return %self : !struct.type<@StructA<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@StructA<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
      function.return
    }
  }
  struct.def @StructB<[]> {
    function.def @compute() -> !struct.type<@StructB<[]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
      %self = struct.new : <@StructB<[]>>
      %0 = function.call @StructA::@compute() : () -> !struct.type<@StructA<[]>>
      function.return %self : !struct.type<@StructB<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@StructB<[]>>) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
      function.return
    }
  }
}
