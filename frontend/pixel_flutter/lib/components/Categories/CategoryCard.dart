import 'package:flutter/material.dart';
import 'package:pixel_flutter/components/Categories/CardName.dart';
import 'package:pixel_flutter/components/Categories/CardDetail.dart';
import 'package:pixel_flutter/models/Category.dart';

class CategoryCard extends StatelessWidget {
  final Category category;

  CategoryCard({this.category});

  Widget build(BuildContext context) {
    return SizedBox(
        width: 280,
        child: Card(
          margin: EdgeInsets.symmetric(horizontal: 25.0, vertical: 60.0),
          elevation: 20.0,
          shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.all(Radius.circular(20.0))),
          child: ClipRRect(
            borderRadius: BorderRadius.all(Radius.circular(20.0)),
            child: Stack(
              children: <Widget>[
                Image.asset('assets/test2.png', fit: BoxFit.fitWidth),
                Positioned(
                  left: 0,
                  bottom: 0,
                  right: 0,
                  child: CardDetail(),
                ),
                Positioned(
                    left: 0,
                    bottom: 70,
                    child: CardName(
                      cardName: 'General',
                    )),
              ],
            ),
          ),
        ));
  }
}

//  Widget build(BuildContext context) {
//    return InkWell(
//        onTap: () => Navigator.of(context).pushNamed('/topics'),
//        child: Card(
//          elevation: 10,
//          child: Stack(
//            alignment: AlignmentDirectional.topCenter,
//            children: <Widget>[
//              Positioned(
//                child: Image.asset('assets/test3.jpg'),
//              ),
//              Positioned(
//                top: 80,
//                child: Text(categoryName,
//                    style: TextStyle(
//                        fontSize: 30,
//                        color: Colors.white,
//                        fontWeight: FontWeight.bold)),
//              ),
//            ],
//          ),
//        ));
//  }
