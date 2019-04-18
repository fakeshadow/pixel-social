import 'package:flutter/material.dart';
import '../components/NavigationBar/TabNavBar.dart';
import '../components/NavigationBar/SliverNavBarExpand.dart';
import '../components/Profile/Me.dart';

class ProfilePage extends StatefulWidget {
  @override
  _ProfilePageState createState() => _ProfilePageState();
}

class _ProfilePageState extends State<ProfilePage> {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: DefaultTabController(
        length: 2,
        child: NestedScrollView(
            headerSliverBuilder: _headerSilverBuilder,
            body: TabBarView(
              children: <Widget>[Me(), Collection()],
            )),
      ),
      bottomNavigationBar: TabNavBar(2),
    );
  }

  List<Widget> _headerSilverBuilder(
      BuildContext context, bool innerBoxIsScrolled) {
    return <Widget>[
      SliverNavBarExpand(),
      SliverPersistentHeader(
        delegate: _SliverAppBarDelegate(
          TabBar(
            labelColor: Colors.black87,
            unselectedLabelColor: Colors.grey,
            tabs: [
              Tab(text: '@me'),
              Tab(text: 'Collection'),
            ],
          ),
        ),
        pinned: true,
        floating: false,
      ),
    ];
  }
}

class _SliverAppBarDelegate extends SliverPersistentHeaderDelegate {
  final TabBar _tabBar;
  _SliverAppBarDelegate(this._tabBar);

  @override
  double get minExtent => _tabBar.preferredSize.height;
  @override
  double get maxExtent => _tabBar.preferredSize.height;

  @override
  Widget build(
      BuildContext context, double shrinkOffset, bool overlapsContent) {
    return new Container(
      color: Colors.blue,
      child: _tabBar,
    );
  }

  @override
  bool shouldRebuild(_SliverAppBarDelegate oldDelegate) {
    return false;
  }
}

class Collection extends StatelessWidget {
  Widget build(BuildContext context) {
    return Container(
      child: Center(child: Text('test test')),
    );
  }
}
