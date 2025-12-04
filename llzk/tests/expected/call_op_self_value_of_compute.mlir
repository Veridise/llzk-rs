module attributes {veridise.lang = "llzk"} {
  struct.def @StructA<[]> {
    function.def @compute() -> !struct.type<@StructA<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@StructA<[]>>
      function.return %self : !struct.type<@StructA<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@StructA<[]>>) attributes {function.allow_constraint} {
      function.return
    }
  }
  struct.def @StructB<[]> {
    function.def @compute() -> !struct.type<@StructB<[]>> attributes {function.allow_witness} {
      %self = struct.new : <@StructB<[]>>
      %0 = function.call @StructA::@compute() : () -> !struct.type<@StructA<[]>>
      function.return %self : !struct.type<@StructB<[]>>
    }
    function.def @constrain(%arg0: !struct.type<@StructB<[]>>) attributes {function.allow_constraint} {
      function.return
    }
  }
}
