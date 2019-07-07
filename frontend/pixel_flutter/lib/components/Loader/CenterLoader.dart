import 'package:flutter/material.dart';

class CenterLoader extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return SliverFillViewport(
      delegate: SliverChildBuilderDelegate((context, index) {
        return Container(
            width: 10,
            child: Center(
              child: CircularProgressIndicator(strokeWidth: 2),
            ));
      }, childCount: 1),
    );
  }
}