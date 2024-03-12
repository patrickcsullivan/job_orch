use async_trait::async_trait;
use tokio::join;

#[async_trait]
pub trait Node: Send + Sync {
    type Dependency: Send + Sync;
    type Value: Send + Sync;
    type Error: Send + Sync;

    async fn drive(&mut self, deps: &Self::Dependency) -> Result<Self::Value, Self::Error>;
}

pub struct And2<N1, N2> {
    n1: N1,
    n2: N2,
}

#[async_trait]
impl<N1, N2> Node for And2<N1, N2>
where
    N1: Node,
    N2: Node<Error = N1::Error>,
{
    type Dependency = (N1::Dependency, N2::Dependency);
    type Value = (N1::Value, N2::Value);
    type Error = N1::Error;

    async fn drive(&mut self, deps: &Self::Dependency) -> Result<Self::Value, Self::Error> {
        let (d1, d2) = deps;
        let (v1, v2) = join!(self.n1.drive(d1), self.n2.drive(d2));
        let v1 = v1?;
        let v2 = v2?;
        Ok((v1, v2))
    }
}

pub struct And3<N1, N2, N3> {
    n1: N1,
    n2: N2,
    n3: N3,
}

#[async_trait]
impl<N1, N2, N3> Node for And3<N1, N2, N3>
where
    N1: Node,
    N2: Node<Error = N1::Error>,
    N3: Node<Error = N1::Error>,
{
    type Dependency = (N1::Dependency, N2::Dependency, N3::Dependency);
    type Value = (N1::Value, N2::Value, N3::Value);
    type Error = N1::Error;

    async fn drive(&mut self, deps: &Self::Dependency) -> Result<Self::Value, Self::Error> {
        let (d1, d2, d3) = deps;
        let (v1, v2, v3) = join!(self.n1.drive(d1), self.n2.drive(d2), self.n3.drive(d3));
        let v1 = v1?;
        let v2 = v2?;
        let v3 = v3?;
        Ok((v1, v2, v3))
    }
}
