define i16 @popcnt(i16) {
  br label %2

  phi i16 [ %0, %1 ], [ %8, %6 ]
  phi i16 [ 0, %1 ], [ %9, %6 ]
  icmp eq i16 %3, 0
  br i1 %5, label %10, label %6

  add i16 %3, -1
  and i16 %3, %7
  add i16 %4, 1
  br label %2

  ret i16 %4
}
