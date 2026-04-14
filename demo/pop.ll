define i16 @popcnt(i16 %x) {
entry:
  br label %while.cond

while.cond:
  %x.addr.0 = phi i16 [ %x, %entry ], [ %and, %while.body ]
  %c.0 = phi i16 [ 0, %entry ], [ %inc, %while.body ]
  %tobool.not = icmp eq i16 %x.addr.0, 0
  br i1 %tobool.not, label %while.end, label %while.body

while.body:
  %sub = add i16 %x.addr.0, -1
  %and = and i16 %x.addr.0, %sub
  %inc = add i16 %c.0, 1
  br label %while.cond

while.end:
  ret i16 %c.0
}
