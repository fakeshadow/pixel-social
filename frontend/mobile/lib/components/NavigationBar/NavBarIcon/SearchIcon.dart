import 'package:flutter/material.dart';

class SearchIcon extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return IconButton(
        onPressed: (){},
        padding: const EdgeInsets.only(top: 13, right: 9, left: 9, bottom: 13),
        icon: Icon(Icons.search));
  }
}
