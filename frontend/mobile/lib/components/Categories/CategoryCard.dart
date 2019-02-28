import 'package:flutter/material.dart';

class CategoryCard extends StatelessWidget {
  final String categoryName;
  final int categoryId;
  CategoryCard({this.categoryName, this.categoryId});

  Widget build(BuildContext context) {
    return InkWell(
        onTap: () => Navigator.of(context).pushNamed('/topics'),
        child: Card(
          elevation: 10,
          child: Stack(
            alignment: AlignmentDirectional.topCenter,
            children: <Widget>[
              Positioned(
                child: Image.asset('assets/test3.jpg'),
              ),
              Positioned(
                top: 80,
                child: Text(categoryName,
                    style: TextStyle(
                        fontSize: 30,
                        color: Colors.white,
                        fontWeight: FontWeight.bold)),
              ),
            ],
          ),
        ));
  }
}
